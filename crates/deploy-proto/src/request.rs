use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanRequest {
    #[serde(default)]
    pub subnet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostKeyRequest {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustHostKeyRequest {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub fingerprint: String,
}

#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub key_path: Option<String>,
    #[serde(default)]
    pub local_keys: bool,
}

impl std::fmt::Debug for Credentials {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Elevation {
    #[default]
    Auto,
    Sudo,
    Su,
}

#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
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
    #[serde(default)]
    pub no_kiosk: bool,
    #[serde(default)]
    pub skip_packages: bool,
    #[serde(default)]
    pub script_path: Option<String>,
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
