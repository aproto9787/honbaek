use crate::domain::ToolCall;
use anyhow::Result;
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use uuid::Uuid;

const TOOL_OUTPUT_LIMIT: usize = 4096;
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct ToolRuntime {
    pub profile: String,
    pub allow_file_read: bool,
    pub allow_file_write: bool,
    pub allow_file_delete: bool,
    pub allow_command_exec: bool,
    pub allow_network: bool,
    command_timeout: Duration,
    network_timeout: Duration,
}

impl ToolRuntime {
    pub fn unbound() -> Self {
        Self::unbound_profile("unbound")
    }

    pub fn unbound_profile(profile: impl Into<String>) -> Self {
        Self {
            profile: profile.into(),
            allow_file_read: true,
            allow_file_write: true,
            allow_file_delete: true,
            allow_command_exec: true,
            allow_network: true,
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            network_timeout: DEFAULT_NETWORK_TIMEOUT,
        }
    }

    pub fn safe(profile: impl Into<String>) -> Self {
        Self {
            profile: profile.into(),
            allow_file_read: true,
            allow_file_write: true,
            allow_file_delete: false,
            allow_command_exec: false,
            allow_network: false,
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            network_timeout: DEFAULT_NETWORK_TIMEOUT,
        }
    }

    pub fn read_file(&self, task_id: Option<Uuid>, path: &Path) -> Result<ToolCall> {
        let input = path.display().to_string();
        if !self.allow_file_read {
            return Ok(self.call(task_id, "file.read", &input, "blocked by profile", false));
        }

        let output = match fs::read_to_string(path) {
            Ok(contents) => compact(&contents),
            Err(error) => format!("failed: {error}"),
        };
        Ok(self.call(
            task_id,
            "file.read",
            &input,
            &output,
            !output.starts_with("failed:"),
        ))
    }

    pub fn write_file(
        &self,
        task_id: Option<Uuid>,
        path: &Path,
        contents: &str,
    ) -> Result<ToolCall> {
        let input = path.display().to_string();
        if !self.allow_file_write {
            return Ok(self.call(task_id, "file.write", &input, "blocked by profile", false));
        }

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            && let Err(error) = fs::create_dir_all(parent)
        {
            let output = format!("failed to create parent directory: {error}");
            return Ok(self.call(task_id, "file.write", &input, &output, false));
        }

        let output = match fs::write(path, contents) {
            Ok(()) => format!("written {} bytes", contents.len()),
            Err(error) => format!("failed: {error}"),
        };
        Ok(self.call(
            task_id,
            "file.write",
            &input,
            &output,
            !output.starts_with("failed:"),
        ))
    }

    pub fn delete_file(&self, task_id: Option<Uuid>, path: &Path) -> Result<ToolCall> {
        let input = path.display().to_string();
        if !self.allow_file_delete {
            return Ok(self.call(task_id, "file.delete", &input, "blocked by profile", false));
        }

        let output = match fs::remove_file(path) {
            Ok(()) => "deleted".to_string(),
            Err(error) => format!("failed: {error}"),
        };
        Ok(self.call(
            task_id,
            "file.delete",
            &input,
            &output,
            !output.starts_with("failed:"),
        ))
    }

    pub fn run_command(
        &self,
        task_id: Option<Uuid>,
        program: &str,
        args: &[&str],
    ) -> Result<ToolCall> {
        let input = format_command(program, args);
        if !self.allow_command_exec {
            return Ok(self.call(task_id, "shell.exec", &input, "blocked by profile", false));
        }

        let mut child = match Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                return Ok(self.call(
                    task_id,
                    "shell.exec",
                    &input,
                    &format!("failed: {error}"),
                    false,
                ));
            }
        };

        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let output = match child.wait_with_output() {
                        Ok(output) => output,
                        Err(error) => {
                            return Ok(self.call(
                                task_id,
                                "shell.exec",
                                &input,
                                &format!("failed to collect output: {error}"),
                                false,
                            ));
                        }
                    };
                    let rendered = render_command_output(&output);
                    return Ok(self.call(
                        task_id,
                        "shell.exec",
                        &input,
                        &rendered,
                        output.status.success(),
                    ));
                }
                Ok(None) if start.elapsed() >= self.command_timeout => {
                    let _ = child.kill();
                    let output = match child.wait_with_output() {
                        Ok(output) => output,
                        Err(error) => {
                            return Ok(self.call(
                                task_id,
                                "shell.exec",
                                &input,
                                &format!("timed out and failed to collect output: {error}"),
                                false,
                            ));
                        }
                    };
                    let rendered = format!(
                        "timed out after {:?}; {}",
                        self.command_timeout,
                        render_command_output(&output)
                    );
                    return Ok(self.call(task_id, "shell.exec", &input, &rendered, false));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(error) => {
                    return Ok(self.call(
                        task_id,
                        "shell.exec",
                        &input,
                        &format!("failed: {error}"),
                        false,
                    ));
                }
            }
        }
    }

    pub fn http_get(&self, task_id: Option<Uuid>, url: &str) -> Result<ToolCall> {
        if !self.allow_network {
            return Ok(self.call(task_id, "network.get", url, "blocked by profile", false));
        }

        let client = match reqwest::blocking::Client::builder()
            .timeout(self.network_timeout)
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                return Ok(self.call(
                    task_id,
                    "network.get",
                    url,
                    &format!("failed: {error}"),
                    false,
                ));
            }
        };
        let output = match client
            .get(url)
            .send()
            .and_then(|response| response.error_for_status())
        {
            Ok(response) => match response.text() {
                Ok(body) => compact(&body),
                Err(error) => format!("failed: {error}"),
            },
            Err(error) => format!("failed: {error}"),
        };
        Ok(self.call(
            task_id,
            "network.get",
            url,
            &output,
            !output.starts_with("failed:"),
        ))
    }

    fn call(
        &self,
        task_id: Option<Uuid>,
        tool: &str,
        input: &str,
        output: &str,
        ok: bool,
    ) -> ToolCall {
        ToolCall {
            id: Uuid::new_v4(),
            task_id,
            tool: tool.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            ok,
            at: Utc::now(),
        }
    }
}

fn format_command(program: &str, args: &[&str]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{program} {}", args.join(" "))
    }
}

fn render_command_output(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    compact(&format!(
        "status={}; stdout={}; stderr={}",
        output.status,
        stdout.trim(),
        stderr.trim()
    ))
}

fn compact(value: &str) -> String {
    let mut chars = value.chars();
    let mut rendered: String = chars.by_ref().take(TOOL_OUTPUT_LIMIT).collect();
    if chars.next().is_some() {
        rendered.push_str("\n[truncated]");
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::ToolRuntime;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;
    use uuid::Uuid;

    #[test]
    fn unbound_delete_records_success() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("delete-me.txt");
        fs::write(&target, "temporary").unwrap();

        let call = ToolRuntime::unbound().delete_file(None, &target).unwrap();

        assert!(call.ok);
        assert_eq!(call.tool, "file.delete");
        assert!(!target.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn safe_runtime_records_blocked_command() {
        let call = ToolRuntime::safe("safe")
            .run_command(None, "sh", &["-c", "echo blocked"])
            .unwrap();

        assert!(!call.ok);
        assert_eq!(call.output, "blocked by profile");
    }

    #[test]
    fn unbound_network_get_records_local_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0; 1024];
            let _ = stream.read(&mut buffer);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\nhonbaek")
                .unwrap();
        });

        let call = ToolRuntime::unbound()
            .http_get(None, &format!("http://{addr}/status"))
            .unwrap();

        handle.join().unwrap();
        assert!(call.ok);
        assert_eq!(call.output, "honbaek");
    }

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("honbaek-tools-test-{}", Uuid::new_v4()))
    }
}
