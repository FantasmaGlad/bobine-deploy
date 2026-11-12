mod auth;
mod channel;
mod connect;
mod host_key;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bobine_deploy_proto::{Credentials, Error};
use russh::client::{self, Handle};
use russh::keys::ssh_key::PublicKey;
use russh::keys::PublicKeyOrCertificate;

pub struct Session {
    handle: Handle<Recorder>,
    host: String,
    port: u16,
    host_key: PublicKey,
}

pub(crate) struct Recorder {
    key: Arc<Mutex<Option<PublicKey>>>,
}

impl client::Handler for Recorder {
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let key = match server_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(cert) => cert.public_key().clone().into(),
        };
        if let Ok(mut slot) = self.key.lock() {
            *slot = Some(key);
        }
        Ok(true)
    }
}

pub async fn open(creds: &Credentials) -> Result<Session, Error> {
    let mut session = Session::connect(&creds.host, creds.port).await?;
    session.require_known_host()?;
    session.authenticate(creds).await?;
    Ok(session)
}

impl Session {
    pub async fn close(self) {
        let _ = self.handle.disconnect(russh::Disconnect::ByApplication, "", "fr").await;
    }
}

fn transport(e: russh::Error) -> Error {
    Error::failed(format!("Erreur SSH : {e}"))
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
