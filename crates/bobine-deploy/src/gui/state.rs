//! État de l'interface, un bloc par écran. Les mots de passe sont effacés de la mémoire à la fermeture.

use bobine_deploy_proto::{
    AppInfo, Channel, Credentials, Device, HostKeyInfo, Inspection, InstallOutcome, InstallProgress, ScanProgress,
};
use zeroize::Zeroize;

use super::message::Screen;

pub struct App {
    pub screen: Screen,
    pub version: Option<AppInfo>,
    pub scan: ScanState,
    pub form: Form,
    pub target: TargetState,
    pub options: Options,
    pub install: InstallState,
}

#[derive(Default)]
pub struct ScanState {
    pub running: bool,
    pub subnet: String,
    pub devices: Vec<Device>,
    pub progress: Option<ScanProgress>,
    pub error: Option<String>,
}

pub struct Form {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub key_path: Option<String>,
    pub local_keys: bool,
}

impl Form {
    pub fn port(&self) -> u16 {
        self.port.trim().parse().unwrap_or(22)
    }

    pub fn credentials(&self) -> Credentials {
        Credentials {
            host: self.host.trim().to_string(),
            port: self.port(),
            username: self.user.trim().to_string(),
            password: Some(self.password.clone()).filter(|p| !p.is_empty()),
            key_path: self.key_path.clone(),
            local_keys: self.local_keys,
        }
    }

    pub fn is_complete(&self) -> bool {
        !self.host.trim().is_empty() && !self.user.trim().is_empty()
    }
}

impl Default for Form {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: "22".to_string(),
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_default(),
            password: String::new(),
            key_path: None,
            local_keys: false,
        }
    }
}

impl Drop for Form {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

/// Clé d'hôte et audit de la cible en cours.
#[derive(Default)]
pub struct TargetState {
    pub busy: bool,
    pub host_key: Option<HostKeyInfo>,
    pub inspection: Option<Inspection>,
    pub error: Option<String>,
}

pub struct Options {
    pub channel: Channel,
    pub kiosk: bool,
    pub skip_packages: bool,
    pub root_password: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            channel: Channel::Stable,
            kiosk: true,
            skip_packages: false,
            root_password: String::new(),
        }
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        self.root_password.zeroize();
    }
}

#[derive(Default)]
pub struct InstallState {
    pub running: bool,
    pub simulated: bool,
    pub progress: Option<InstallProgress>,
    pub log: Vec<String>,
    pub outcome: Option<Result<InstallOutcome, String>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            screen: Screen::Scan,
            version: None,
            scan: ScanState::default(),
            form: Form::default(),
            target: TargetState::default(),
            options: Options::default(),
            install: InstallState::default(),
        }
    }
}
