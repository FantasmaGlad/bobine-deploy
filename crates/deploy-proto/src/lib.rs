//! Contrat de Bobine Deploy : une commande décrit une opération, le moteur l'exécute en émettant des événements et rend
//! un résultat. L'interface graphique, la ligne de commande et l'interface d'automatisation (JSON-RPC sur stdio) ne
//! font que traduire vers et depuis ces types : aucune logique métier ne vit dans une façade.
//!
//! [`CATALOG`] est la liste unique des commandes ; un test de l'application vérifie que chaque façade les expose toutes.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Version du contrat, annoncée par `version` et par l'interface d'automatisation. Elle change quand un champ change
/// de sens ou disparaît ; un champ ajouté (avec valeur par défaut) ne la change pas.
pub const PROTOCOL_VERSION: u32 = 1;

/// Description d'une commande du catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    /// Nom stable : sous-commande de la ligne de commande et méthode de l'interface d'automatisation.
    pub name: &'static str,
    pub summary: &'static str,
}

/// Toutes les commandes, dans l'ordre d'un déploiement.
pub const CATALOG: &[CommandSpec] = &[
    CommandSpec {
        name: "scan",
        summary: "Chercher les machines du réseau local (ou d'un réseau donné).",
    },
    CommandSpec {
        name: "host-key",
        summary: "Empreinte et statut de la clé d'hôte SSH de la cible, sans s'authentifier.",
    },
    CommandSpec {
        name: "trust-host-key",
        summary: "Enregistrer la clé d'hôte de la cible si son empreinte est celle donnée.",
    },
    CommandSpec {
        name: "inspect",
        summary: "Se connecter en SSH et auditer la cible (système, architecture, élévation, matériel).",
    },
    CommandSpec {
        name: "install",
        summary: "Installer Bobine sur la cible.",
    },
    CommandSpec {
        name: "version",
        summary: "Version de Bobine Deploy et du contrat.",
    },
];

/// Une opération demandée au moteur.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub enum Command {
    Scan(ScanRequest),
    HostKey(HostKeyRequest),
    TrustHostKey(TrustHostKeyRequest),
    Inspect(Credentials),
    Install(InstallRequest),
    Version,
}

impl Command {
    /// Nom du catalogue correspondant.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Scan(_) => "scan",
            Self::HostKey(_) => "host-key",
            Self::TrustHostKey(_) => "trust-host-key",
            Self::Inspect(_) => "inspect",
            Self::Install(_) => "install",
            Self::Version => "version",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanRequest {
    /// Adresse ou réseau à sonder (`192.168.2.10`, `192.168.2.0/24`) ; absent : réseaux des interfaces actives.
    #[serde(default)]
    pub subnet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostKeyRequest {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustHostKeyRequest {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    /// Empreinte confirmée par l'utilisateur (`SHA256:…`).
    pub fingerprint: String,
}

/// Identifiants SSH, effacés de la mémoire à la fin de leur usage.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Credentials {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    /// Clé privée choisie explicitement : essayée avant le mot de passe.
    #[serde(default)]
    pub key_path: Option<String>,
    /// Accord de l'utilisateur pour présenter l'agent SSH et les clés usuelles de `~/.ssh`.
    #[serde(default)]
    pub local_keys: bool,
}

impl std::fmt::Debug for Credentials {
    /// Jamais de mot de passe dans un journal ou un message d'erreur.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "***"))
            .field("key_path", &self.key_path)
            .field("local_keys", &self.local_keys)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
    Canary,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Canary => "canary",
        }
    }
}

/// Élévation des privilèges sur la cible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Elevation {
    /// Déterminée par l'audit (`inspect`), comme l'interface.
    #[default]
    Auto,
    Sudo,
    /// `su -` : mot de passe root requis.
    Su,
}

#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct InstallRequest {
    #[zeroize(skip)]
    pub credentials: Credentials,
    #[serde(default)]
    #[zeroize(skip)]
    pub channel: Channel,
    #[serde(default)]
    #[zeroize(skip)]
    pub elevation: Elevation,
    #[serde(default)]
    pub root_password: Option<String>,
    /// Sans le kiosque plein écran sur la cible.
    #[serde(default)]
    pub no_kiosk: bool,
    /// Sans installer les paquets système (déjà présents).
    #[serde(default)]
    pub skip_packages: bool,
    /// Script d'installation déjà présent sur la cible, au lieu du script embarqué. Réservé à la ligne de commande :
    /// l'interface et l'interface d'automatisation le refusent.
    #[serde(default)]
    pub script_path: Option<String>,
    /// Rejoue une installation enregistrée sans toucher la cible.
    #[serde(default)]
    pub simulate: bool,
}

impl std::fmt::Debug for InstallRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstallRequest")
            .field("credentials", &self.credentials)
            .field("channel", &self.channel)
            .field("elevation", &self.elevation)
            .field("root_password", &self.root_password.as_ref().map(|_| "***"))
            .field("no_kiosk", &self.no_kiosk)
            .field("skip_packages", &self.skip_packages)
            .field("script_path", &self.script_path)
            .field("simulate", &self.simulate)
            .finish()
    }
}

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

/// Événement émis pendant une commande.
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
    /// Réseaux balayés, par exemple `192.168.1.0/24 (wlp3s0)`.
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

/// Cause d'un échec : elle donne le code de sortie de la ligne de commande et le code d'erreur JSON-RPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Nom introuvable, connexion refusée ou expirée, port sans serveur SSH.
    Unreachable,
    /// Le serveur SSH répond mais refuse la clé d'hôte ou l'authentification.
    Refused,
    /// Demande incomplète ou contradictoire : rien n'a été tenté.
    Usage,
    /// Tout le reste.
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn unreachable(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unreachable, message)
    }
    pub fn refused(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Refused, message)
    }
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Usage, message)
    }
    pub fn failed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Failed, message)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub const fn default_ssh_port() -> u16 {
    22
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_variant_is_in_the_catalog() {
        let commands = [
            Command::Scan(ScanRequest::default()),
            Command::HostKey(HostKeyRequest {
                host: "h".into(),
                port: 22,
            }),
            Command::TrustHostKey(TrustHostKeyRequest {
                host: "h".into(),
                port: 22,
                fingerprint: "f".into(),
            }),
            Command::Inspect(credentials()),
            Command::Install(InstallRequest {
                credentials: credentials(),
                channel: Channel::Stable,
                elevation: Elevation::Auto,
                root_password: None,
                no_kiosk: false,
                skip_packages: false,
                script_path: None,
                simulate: true,
            }),
            Command::Version,
        ];
        let names: Vec<&str> = commands.iter().map(Command::name).collect();
        let catalog: Vec<&str> = CATALOG.iter().map(|c| c.name).collect();
        assert_eq!(names, catalog);
    }

    #[test]
    fn commands_use_the_catalog_name_on_the_wire() {
        let json = serde_json::to_value(Command::TrustHostKey(TrustHostKeyRequest {
            host: "bobine.local".into(),
            port: 22,
            fingerprint: "SHA256:x".into(),
        }))
        .unwrap();
        assert_eq!(json["command"], "trust-host-key");
        let parsed: Command = serde_json::from_str(r#"{"command":"inspect","host":"h","username":"u"}"#).unwrap();
        assert!(matches!(parsed, Command::Inspect(ref c) if c.port == 22 && !c.local_keys));
    }

    #[test]
    fn secrets_never_appear_in_debug_output() {
        let mut creds = credentials();
        creds.password = Some("hunter2".into());
        assert!(!format!("{creds:?}").contains("hunter2"));
    }

    fn credentials() -> Credentials {
        Credentials {
            host: "h".into(),
            port: 22,
            username: "u".into(),
            password: None,
            key_path: None,
            local_keys: false,
        }
    }
}
