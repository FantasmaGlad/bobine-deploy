//! Paramètres des commandes. Les secrets sont effacés de la mémoire et jamais affichés par `Debug`.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

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

pub const fn default_ssh_port() -> u16 {
    22
}
