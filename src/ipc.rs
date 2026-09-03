//! The shared communication contract for the entire MITOS ecosystem.
//! Every crate (terminal, shell, file-manager, system-monitor, settings)
//! depends on this file so messages always stay in sync.

use serde::{Deserialize, Serialize};
use std::io::Write;

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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcResponse {
    AutoCompleteResult { suggestions: Vec<String> },
    BufferData { text: String },
    Ack,
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
