//! Contrat de Bobine Deploy : une commande décrit une opération, le moteur l'exécute en émettant des événements et rend
//! un résultat. L'interface graphique, la ligne de commande et l'interface d'automatisation (JSON-RPC sur stdio) ne
//! font que traduire vers et depuis ces types : aucune logique métier ne vit dans une façade.
//!
//! [`CATALOG`] est la liste unique des commandes ; un test de l'application vérifie que chaque façade les expose toutes.

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
