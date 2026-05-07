use crate::domain::{InspectState, Kaeyi, KaeyiSeverity};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcRequest {
    Ping,
    Awaken {
        name: String,
        profile: String,
    },
    Assign {
        hon: Option<String>,
        task: String,
    },
    Inspect,
    KaeyiList,
    KaeyiInspect {
        id: Uuid,
    },
    KaeyiScan,
    KaeyiRecord {
        title: String,
        evidence: String,
        severity: KaeyiSeverity,
    },
    KaeyiContain {
        id: Uuid,
        note: String,
    },
    KaeyiResolve {
        id: Uuid,
        note: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok { message: String },
    Awakened { hon_id: Uuid, message: String },
    Assigned { task_id: Uuid, message: String },
    Inspect { state: Box<InspectState> },
    KaeyiList { kaeyi: Vec<Kaeyi> },
    KaeyiInspect { kaeyi: Box<Kaeyi> },
    KaeyiChanged { kaeyi: Box<Kaeyi>, message: String },
    Error { message: String },
}

pub fn send(socket: &Path, request: &IpcRequest) -> Result<IpcResponse> {
    let mut stream = UnixStream::connect(socket)
        .with_context(|| format!("failed to connect daemon socket {}", socket.display()))?;
    let raw = serde_json::to_string(request)?;
    stream.write_all(raw.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response = serde_json::from_str(line.trim())?;
    Ok(response)
}
