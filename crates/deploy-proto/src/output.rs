//! Résultats des commandes.

use serde::{Deserialize, Serialize};

use crate::Elevation;

/// Résultat d'une commande réussie.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum Output {
    Scan { devices: Vec<Device> },
    HostKey(HostKeyInfo),
    Inspect(Inspection),
    Install(InstallOutcome),
    Version(AppInfo),
}

/// Un appareil détecté sur le réseau, pas nécessairement une cible Bobine : tout ce qui répond est listé.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Device {
    pub ip: String,
    pub hostname: Option<String>,
    pub ssh_open: bool,
    /// Le port de Bobine répond et `/api/health` a la forme d'un backend Bobine.
    pub bobine_open: bool,
    pub open_ports: Vec<u16>,
    pub os_hint: Option<String>,
    pub is_wyse_or_bobine: bool,
    #[serde(default)]
    pub mac: Option<String>,
    /// `ports`, `arp` ou `mdns`.
    #[serde(default)]
    pub seen_via: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostKeyStatus {
    Known,
    Unknown,
    Changed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostKeyInfo {
    pub host: String,
    pub port: u16,
    /// Empreinte au format d'OpenSSH (`SHA256:…`), comparable à `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub`.
    pub fingerprint: String,
    pub key_type: String,
    pub status: HostKeyStatus,
    pub known_hosts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Inspection {
    pub host: String,
    pub os_name: String,
    pub is_debian: bool,
    pub is_debian_13: bool,
    pub arch: String,
    pub is_amd64: bool,
    pub sudo_installed: bool,
    pub user_is_sudoer: bool,
    /// `sudo` ou `su`.
    pub elevation: Elevation,
    pub needs_root_password: bool,
    pub cpu_model: String,
    pub memory_total_mb: u64,
    pub disk_free_gb: u64,
    pub gpu_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallOutcome {
    pub access_url: Option<String>,
    pub access_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppInfo {
    pub version: String,
    pub commit: String,
    pub protocol: u32,
}
