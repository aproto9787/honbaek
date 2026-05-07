use crate::config::AppPaths;
use crate::domain::RuntimeEvent;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct Journal {
    paths: AppPaths,
}

impl Journal {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn append(&self, event: &RuntimeEvent) -> Result<()> {
        self.paths.ensure()?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.paths.journal)
            .with_context(|| format!("failed to open {}", self.paths.journal.display()))?;
        let line = serde_json::to_string(event)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }
}
