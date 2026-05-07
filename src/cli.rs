use crate::config::{AppPaths, Config};
use crate::domain::{DEFAULT_HON_NAME, DEFAULT_PROFILE_NAME, Kaeyi, KaeyiSeverity};
use crate::ipc::{IpcRequest, IpcResponse};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "honbaek", version, about = "혼백강령 local autonomous runtime")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Awaken {
        #[arg(long, default_value = DEFAULT_HON_NAME)]
        name: String,
        #[arg(long, default_value = DEFAULT_PROFILE_NAME)]
        profile: String,
    },
    Assign {
        task: String,
        #[arg(long)]
        hon: Option<String>,
    },
    Watch {
        #[arg(long)]
        once: bool,
    },
    Inspect {
        #[arg(long)]
        json: bool,
    },
    Kaeyi {
        #[command(subcommand)]
        command: KaeyiCommand,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    Service {
        #[command(subcommand)]
        command: crate::service::ServiceCommand,
    },
    Completions {
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    Run,
    Stop,
}

#[derive(Debug, Subcommand)]
enum KaeyiCommand {
    List,
    Inspect {
        id: Uuid,
    },
    Scan,
    Record {
        title: String,
        #[arg(long)]
        evidence: String,
        #[arg(long, default_value = "warning")]
        severity: KaeyiSeverity,
    },
    Contain {
        id: Uuid,
        #[arg(long)]
        note: String,
    },
    Resolve {
        id: Uuid,
        #[arg(long)]
        note: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;
    let config = Config::load(&paths)?;

    match cli.command {
        Command::Awaken { name, profile } => {
            print_response(crate::daemon::awaken(&paths, &name, &profile)?)
        }
        Command::Assign { task, hon } => {
            crate::daemon::ensure_running(&paths)?;
            let response = crate::ipc::send(&paths.socket, &IpcRequest::Assign { hon, task })?;
            print_response(response)
        }
        Command::Watch { once } => {
            crate::daemon::ensure_running(&paths)?;
            crate::tui::watch(&paths, once)
        }
        Command::Inspect { json } => {
            crate::daemon::ensure_running(&paths)?;
            let response = crate::ipc::send(&paths.socket, &IpcRequest::Inspect)?;
            match response {
                IpcResponse::Inspect { state } if json => {
                    println!("{}", serde_json::to_string_pretty(&state)?);
                    Ok(())
                }
                IpcResponse::Inspect { state } => {
                    crate::tui::print_inspect(&state);
                    Ok(())
                }
                other => print_response(other),
            }
        }
        Command::Kaeyi { command } => {
            crate::daemon::ensure_running(&paths)?;
            let request = match command {
                KaeyiCommand::List => IpcRequest::KaeyiList,
                KaeyiCommand::Inspect { id } => IpcRequest::KaeyiInspect { id },
                KaeyiCommand::Scan => IpcRequest::KaeyiScan,
                KaeyiCommand::Record {
                    title,
                    evidence,
                    severity,
                } => IpcRequest::KaeyiRecord {
                    title,
                    evidence,
                    severity,
                },
                KaeyiCommand::Contain { id, note } => IpcRequest::KaeyiContain { id, note },
                KaeyiCommand::Resolve { id, note } => IpcRequest::KaeyiResolve { id, note },
            };
            let response = crate::ipc::send(&paths.socket, &request)?;
            print_response(response)
        }
        Command::Daemon { command } => match command {
            DaemonCommand::Run => crate::daemon::run_foreground(paths, config),
            DaemonCommand::Stop => {
                let response = crate::ipc::send(&paths.socket, &IpcRequest::Shutdown)
                    .context("failed to ask daemon to stop")?;
                print_response(response)
            }
        },
        Command::Service { command } => crate::service::run(command, &paths),
        Command::Completions { shell } => crate::completions::print(shell),
    }
}

fn print_response(response: IpcResponse) -> Result<()> {
    match response {
        IpcResponse::Ok { message }
        | IpcResponse::Awakened { message, .. }
        | IpcResponse::Assigned { message, .. }
        | IpcResponse::KaeyiChanged { message, .. } => {
            println!("{message}");
            Ok(())
        }
        IpcResponse::Inspect { state } => {
            crate::tui::print_inspect(&state);
            Ok(())
        }
        IpcResponse::KaeyiList { kaeyi } => {
            print_kaeyi_list(&kaeyi);
            Ok(())
        }
        IpcResponse::KaeyiInspect { kaeyi } => {
            print_kaeyi_detail(&kaeyi);
            Ok(())
        }
        IpcResponse::Error { message } => anyhow::bail!("{message}"),
    }
}

fn print_kaeyi_list(records: &[Kaeyi]) {
    println!("怪異 list");
    if records.is_empty() {
        println!("- none");
        return;
    }
    for record in records {
        println!(
            "- {} [{} {}] {} source={} updated={}",
            record.id,
            record.severity,
            record.state,
            record.title,
            record.source_kind,
            record.updated_at
        );
    }
}

fn print_kaeyi_detail(record: &Kaeyi) {
    println!("怪異 inspect");
    println!("id: {}", record.id);
    println!("title: {}", record.title);
    println!("source: {}", record.source_kind);
    println!("severity: {}", record.severity);
    println!("state: {}", record.state);
    if let Some(task_id) = record.task_id {
        println!("task: {task_id}");
    }
    println!("evidence: {}", record.evidence);
    if let Some(note) = &record.containment_note {
        println!("containment: {note}");
    }
    if let Some(note) = &record.resolution_note {
        println!("resolution: {note}");
    }
    println!("created: {}", record.created_at);
    println!("updated: {}", record.updated_at);
}
