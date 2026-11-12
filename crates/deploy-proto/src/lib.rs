mod error;
mod event;
mod output;
mod request;

use serde::{Deserialize, Serialize};

pub use error::{Error, ErrorKind};
pub use event::{Event, InstallPhase, InstallProgress, ScanProgress};
pub use output::{AppInfo, Device, HostKeyInfo, HostKeyStatus, Inspection, InstallOutcome, Output};
pub use request::{
    default_ssh_port, Channel, Credentials, Elevation, HostKeyRequest, InstallRequest, ScanRequest, TrustHostKeyRequest,
};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
}

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
