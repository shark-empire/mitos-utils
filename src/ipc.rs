// mitos-utils/src/ipc.rs
use serde::{Deserialize, Serialize};

// The universal envelope for all MITOS IPC communication
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcRequest {
    // Terminal -> File Manager
    AutoCompletePath { partial_path: String },
    
    // System Monitor -> Terminal
    GetTerminalBuffer,
    
    // Settings -> Terminal
    ThemeChanged { bg: [u8; 3], fg: [u8; 3] },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum IpcResponse {
    AutoCompleteResult { suggestions: Vec<String> },
    BufferData { text: String },
    Ack,
}
