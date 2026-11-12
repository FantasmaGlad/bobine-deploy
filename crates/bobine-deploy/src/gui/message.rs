use std::path::PathBuf;

use bobine_deploy_proto::{Channel, Error, Event, Output};

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
    SubnetChanged(String),
    StartScan,
    SelectDevice(String),
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
    ChannelSelected(Channel),
    KioskToggled(bool),
    SkipPackagesToggled(bool),
    RootPasswordChanged(String),
    StartInstall,
    Simulate,
    CopyLog,
}
