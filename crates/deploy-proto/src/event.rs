use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", content = "data", rename_all = "kebab-case")]
pub enum Event {
    ScanProgress(ScanProgress),
    ScanDevice(Device),
    InstallProgress(InstallProgress),
    InstallLog(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanProgress {
    pub done: usize,
    pub total: usize,
    pub found: usize,
    pub networks: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    NotStarted,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallProgress {
    pub pct: u8,
    pub step_index: u32,
    pub total_steps: u32,
    pub step_title: String,
    pub step_slug: String,
    pub phase: InstallPhase,
    pub error_message: Option<String>,
    pub access_url: Option<String>,
    pub access_port: Option<u16>,
}
