use crate::cli::Cli;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

pub fn print(shell: Shell) -> anyhow::Result<()> {
    let mut command = Cli::command();
    generate(shell, &mut command, "honbaek", &mut io::stdout());
    Ok(())
}
