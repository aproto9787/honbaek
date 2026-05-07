use crate::config::AppPaths;
use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Clone, Subcommand)]
pub enum ServiceCommand {
    Print,
}

pub fn run(command: ServiceCommand, paths: &AppPaths) -> Result<()> {
    match command {
        ServiceCommand::Print => {
            println!("{}", render_unit(paths));
            Ok(())
        }
    }
}

pub fn render_unit(paths: &AppPaths) -> String {
    format!(
        r#"[Unit]
Description=혼백강령 local autonomous runtime daemon
After=network.target

[Service]
Type=simple
ExecStart={exe} daemon run
Restart=on-failure
Environment=HONBAEK_HOME={home}

[Install]
WantedBy=default.target
"#,
        exe = std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "honbaek".to_string()),
        home = paths.home.display()
    )
}
