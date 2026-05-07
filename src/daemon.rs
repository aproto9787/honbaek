use crate::config::{AppPaths, Config};
use crate::domain::{AutonomyMode, Concept, Kaeyi, KaeyiState};
use crate::executor::{emit, execute_local_repo_task};
use crate::ipc::{IpcRequest, IpcResponse};
use crate::journal::Journal;
use crate::provider::{OpenAiCompatibleProvider, ProviderAdapter};
use crate::storage::Store;
use anyhow::{Context, Result, bail};
use serde_json::json;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn ensure_running(paths: &AppPaths) -> Result<()> {
    paths.ensure()?;
    if crate::ipc::send(&paths.socket, &IpcRequest::Ping).is_ok() {
        return Ok(());
    }

    if paths.socket.exists() {
        let _ = fs::remove_file(&paths.socket);
    }

    let exe = std::env::current_exe().context("failed to find current executable")?;
    let mut command = Command::new(exe);
    command
        .arg("daemon")
        .arg("run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    command.spawn().context("failed to spawn honbaek daemon")?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if crate::ipc::send(&paths.socket, &IpcRequest::Ping).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("daemon did not become ready within 5 seconds")
}

pub fn run_foreground(paths: AppPaths, config: Config) -> Result<()> {
    paths.ensure()?;
    if paths.socket.exists() {
        fs::remove_file(&paths.socket)
            .with_context(|| format!("failed to remove {}", paths.socket.display()))?;
    }
    fs::write(&paths.pid, std::process::id().to_string())?;
    let listener = UnixListener::bind(&paths.socket)
        .with_context(|| format!("failed to bind {}", paths.socket.display()))?;

    let provider = OpenAiCompatibleProvider::from_config(&config);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_stream(&paths, &provider, stream) {
                    eprintln!("honbaek daemon error: {error:?}");
                }
            }
            Err(error) => eprintln!("honbaek daemon accept error: {error:?}"),
        }
    }
    Ok(())
}

fn handle_stream(
    paths: &AppPaths,
    provider: &dyn ProviderAdapter,
    mut stream: UnixStream,
) -> Result<()> {
    let mut line = String::new();
    {
        let mut reader = BufReader::new(&stream);
        reader.read_line(&mut line)?;
    }

    let request: IpcRequest = serde_json::from_str(line.trim())?;
    let should_shutdown = matches!(request, IpcRequest::Shutdown);
    let response =
        handle_request(paths, provider, request).unwrap_or_else(|error| IpcResponse::Error {
            message: error.to_string(),
        });
    let raw = serde_json::to_string(&response)?;
    stream.write_all(raw.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    if should_shutdown {
        std::process::exit(0);
    }
    Ok(())
}

fn handle_request(
    paths: &AppPaths,
    provider: &dyn ProviderAdapter,
    request: IpcRequest,
) -> Result<IpcResponse> {
    let store = Store::open(paths)?;
    let journal = Journal::new(paths.clone());
    match request {
        IpcRequest::Ping => Ok(IpcResponse::Ok {
            message: "honbaek daemon is alive".to_string(),
        }),
        IpcRequest::Awaken { name, profile } => {
            store.ensure_profile(&profile, AutonomyMode::Unbound)?;
            let hon = store.ensure_hon(&name, &profile)?;
            emit(
                &store,
                &journal,
                Concept::Hon,
                "hon.awakened",
                &format!("魂 {} awakened with profile {}", hon.name, hon.profile),
                None,
                json!({ "hon_id": hon.id, "profile": hon.profile }),
            )?;
            Ok(IpcResponse::Awakened {
                hon_id: hon.id,
                message: format!("魂 {} awakened", hon.name),
            })
        }
        IpcRequest::Assign { hon, task } => {
            let hon = match hon {
                Some(name) => store
                    .find_hon_by_name(&name)?
                    .context("requested 魂 does not exist")?,
                None => store
                    .default_hon()?
                    .context("no active 魂 exists; run honbaek awaken first")?,
            };
            let task_record = store.insert_task(hon.id, &task)?;
            execute_local_repo_task(paths, &store, &journal, provider, &task_record)?;
            Ok(IpcResponse::Assigned {
                task_id: task_record.id,
                message: format!("task assigned to 魂 {}", hon.name),
            })
        }
        IpcRequest::KaeyiList => Ok(IpcResponse::KaeyiList {
            kaeyi: store.list_kaeyi(100)?,
        }),
        IpcRequest::KaeyiInspect { id } => {
            let kaeyi = store
                .get_kaeyi(id)?
                .with_context(|| format!("怪異 {id} does not exist"))?;
            Ok(IpcResponse::KaeyiInspect {
                kaeyi: Box::new(kaeyi),
            })
        }
        IpcRequest::KaeyiScan => {
            let findings = store.scan_kaeyi()?;
            for finding in &findings {
                emit_kaeyi_event(
                    &store,
                    &journal,
                    &finding.kaeyi,
                    if finding.created {
                        "kaeyi.discovered"
                    } else {
                        "kaeyi.observed"
                    },
                    if finding.created {
                        "怪異 discovered by local scan"
                    } else {
                        "怪異 observed by local scan"
                    },
                )?;
            }
            Ok(IpcResponse::KaeyiList {
                kaeyi: findings.into_iter().map(|finding| finding.kaeyi).collect(),
            })
        }
        IpcRequest::KaeyiRecord {
            title,
            evidence,
            severity,
        } => {
            let kaeyi = store.create_kaeyi(&title, "manual", severity, None, &evidence)?;
            emit_kaeyi_event(
                &store,
                &journal,
                &kaeyi,
                "kaeyi.recorded",
                "怪異 manually recorded",
            )?;
            Ok(IpcResponse::KaeyiChanged {
                message: format!("怪異 {} recorded", kaeyi.id),
                kaeyi: Box::new(kaeyi),
            })
        }
        IpcRequest::KaeyiContain { id, note } => {
            let kaeyi = store.update_kaeyi_state(id, KaeyiState::Contained, &note)?;
            emit_kaeyi_event(
                &store,
                &journal,
                &kaeyi,
                "kaeyi.contained",
                "怪異 sealed for observation",
            )?;
            Ok(IpcResponse::KaeyiChanged {
                message: format!("怪異 {} sealed as {}", kaeyi.id, kaeyi.state),
                kaeyi: Box::new(kaeyi),
            })
        }
        IpcRequest::KaeyiResolve { id, note } => {
            let kaeyi = store.update_kaeyi_state(id, KaeyiState::Resolved, &note)?;
            emit_kaeyi_event(
                &store,
                &journal,
                &kaeyi,
                "kaeyi.resolved",
                "怪異 resolved by local audit",
            )?;
            Ok(IpcResponse::KaeyiChanged {
                message: format!("怪異 {} resolved as {}", kaeyi.id, kaeyi.state),
                kaeyi: Box::new(kaeyi),
            })
        }
        IpcRequest::Inspect => Ok(IpcResponse::Inspect {
            state: Box::new(store.inspect_state(provider.baek())?),
        }),
        IpcRequest::Shutdown => {
            if store.has_work_history()? {
                let finding = store.upsert_kaeyi(
                    "Daemon shutdown with work history",
                    "daemon.shutdown",
                    crate::domain::KaeyiSeverity::Low,
                    None,
                    "命 history exists; daemon shutdowns should be observable continuity events",
                )?;
                emit_kaeyi_event(
                    &store,
                    &journal,
                    &finding.kaeyi,
                    if finding.created {
                        "kaeyi.discovered"
                    } else {
                        "kaeyi.observed"
                    },
                    "怪異 observed daemon shutdown continuity tension",
                )?;
            }
            emit(
                &store,
                &journal,
                Concept::Myeong,
                "daemon.shutdown",
                "honbaek daemon received shutdown",
                None,
                json!({}),
            )?;
            Ok(IpcResponse::Ok {
                message: "honbaek daemon stopped".to_string(),
            })
        }
    }
}

pub fn awaken(paths: &AppPaths, name: &str, profile: &str) -> Result<IpcResponse> {
    ensure_running(paths)?;
    crate::ipc::send(
        &paths.socket,
        &IpcRequest::Awaken {
            name: name.to_string(),
            profile: profile.to_string(),
        },
    )
}

fn emit_kaeyi_event(
    store: &Store,
    journal: &Journal,
    kaeyi: &Kaeyi,
    kind: &str,
    message: &str,
) -> Result<()> {
    emit(
        store,
        journal,
        Concept::Kaeyi,
        kind,
        message,
        kaeyi.task_id,
        json!({
            "kaeyi_id": kaeyi.id,
            "title": kaeyi.title,
            "source_kind": kaeyi.source_kind,
            "severity": kaeyi.severity.to_string(),
            "state": kaeyi.state.to_string(),
        }),
    )?;
    Ok(())
}
