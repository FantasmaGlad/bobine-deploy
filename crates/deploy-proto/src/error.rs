//! Erreur d'une commande et sa cause.

use serde::{Deserialize, Serialize};

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
