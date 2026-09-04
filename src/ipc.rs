//! The shared communication contract for the entire MITOS ecosystem.
//! Every crate (terminal, shell, file-manager, system-monitor, settings)
//! depends on this file so messages always stay in sync.
//!
//! Two transports live here:
//!  * MROP — MITOS Rich Output Protocol (shell ➜ terminal, over stdout/PTY)
//!  * IPC  — length-prefixed JSON over Unix Domain Sockets (daemon ⇄ terminal)

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::Write;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

// ------------------------------------------------------------------
// MROP — MITOS Rich Output Protocol (Shell ➔ Terminal via stdout)
// ------------------------------------------------------------------

pub const OSC_WIDGET: &str = "MITOS_WIDGET";
pub const OSC_NEW_BLOCK: &str = "MITOS_NEW_BLOCK";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum RichWidget {
    #[serde(rename = "button")]
    Button { label: String, cmd: String },

    #[serde(rename = "progress")]
    Progress { percent: f32, color: Option<String> },

    #[serde(rename = "sparkline")]
    Sparkline { data: Vec<f32> },
}

impl RichWidget {
    /// Serialize the widget into a full MROP escape sequence.
    pub fn to_osc(&self) -> String {
        format!(
            "\x1b]{};{}\x07",
            OSC_WIDGET,
            serde_json::to_string(self).unwrap_or_default()
        )
    }

    /// Print the widget straight to stdout (used by mitos-shell & CLI tools).
    pub fn emit(&self) {
        print!("{}", self.to_osc());
        let _ = std::io::stdout().flush();
    }
}

/// Tell the terminal: "previous command finished — close the card".
pub fn emit_new_block(prompt: &str) {
    print!("\x1b]{};{}\x07", OSC_NEW_BLOCK, prompt);
    let _ = std::io::stdout().flush();
}

// ------------------------------------------------------------------
// IPC envelope (Daemon ⇄ Terminal via Unix Domain Sockets)
// ------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcRequest {
    /// Terminal ➔ File Manager: ghost-text autocomplete
    AutoCompletePath { partial_path: String },

    /// System Monitor ➔ Terminal: scrape visible buffer
    GetTerminalBuffer,

    /// Settings ➔ Terminal: live theme swap
    ThemeChanged { bg: [u8; 3], fg: [u8; 3] },

    /// System Monitor ➔ Terminal: render a rich widget inline (MROP over IPC)
    InjectWidget { widget: RichWidget },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcResponse {
    AutoCompleteResult {
        suggestions: Vec<String>,
    },
    BufferData {
        pid: u32,
        prompt: String,
        text: String,
    },
    Ack,
    Error {
        message: String,
    },
}

// ------------------------------------------------------------------
// Socket paths (single source of truth — no hardcoded strings elsewhere)
// ------------------------------------------------------------------

fn runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string())
}

pub fn file_manager_socket() -> String {
    format!("{}/mitos-filemanager.sock", runtime_dir())
}

pub fn terminal_socket(pid: u32) -> String {
    format!("{}/mitos-term-{}.sock", runtime_dir(), pid)
}

/// Discover every running mitos-terminal instance by scanning the runtime dir.
pub fn list_terminal_sockets() -> Vec<(u32, String)> {
    let dir = runtime_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(pid) = name
                .strip_prefix("mitos-term-")
                .and_then(|s| s.strip_suffix(".sock"))
                .and_then(|s| s.parse::<u32>().ok())
            {
                out.push((pid, format!("{}/{}", dir, name)));
            }
        }
    }
    out.sort_by_key(|(pid, _)| *pid);
    out
}

// ------------------------------------------------------------------
// Framing: [u32 LE length][JSON payload]
// Prevents TCP/Unix socket stream fragmentation issues.
// ------------------------------------------------------------------

pub async fn ipc_send<T: Serialize>(stream: &mut UnixStream, msg: &T) -> std::io::Result<()> {
    let json = serde_json::to_vec(msg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    stream.write_all(&(json.len() as u32).to_le_bytes()).await?;
    stream.write_all(&json).await?;
    Ok(())
}

pub async fn ipc_recv<T: DeserializeOwned>(stream: &mut UnixStream) -> std::io::Result<Option<T>> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "ipc message too large",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf)
        .map(Some)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
