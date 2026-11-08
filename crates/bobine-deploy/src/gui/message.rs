//! Messages de l'interface.

use std::path::PathBuf;

use bobine_deploy_proto::{Channel, Error, Event, Output};

/// Commande en cours, pour attribuer son résultat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    Version,
    Scan,
    HostKey,
    TrustHostKey,
    Inspect,
    Install,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Scan,
    Connect,
    Install,
}

#[derive(Debug, Clone)]
pub enum Message {
    Event(Event),
    Finished(Job, Result<Output, Error>),
    Show(Screen),
    // Recherche
    SubnetChanged(String),
    StartScan,
    SelectDevice(String),
    // Connexion
    HostChanged(String),
    PortChanged(String),
    UserChanged(String),
    PasswordChanged(String),
    PickKey,
    KeyPicked(Option<PathBuf>),
    ClearKey,
    LocalKeysToggled(bool),
    CheckHostKey,
    TrustHostKey,
    // Installation
    ChannelSelected(Channel),
    KioskToggled(bool),
    SkipPackagesToggled(bool),
    RootPasswordChanged(String),
    StartInstall,
    Simulate,
    CopyLog,
}
