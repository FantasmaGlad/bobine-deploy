//! Clé d'hôte : `~/.ssh/known_hosts` (ou `BOBINE_SSH_KNOWN_HOSTS`), au format d'OpenSSH. Une clé inconnue n'est
//! enregistrée qu'avec son empreinte confirmée ; une clé changée est toujours refusée, car une machine qui usurpe la
//! borne sur un Wi-Fi partagé recevrait sinon les mots de passe.

use std::path::PathBuf;

use bobine_deploy_proto::{Error, HostKeyInfo, HostKeyStatus};
use russh::keys::ssh_key::HashAlg;

use super::{home_dir, Session};

impl Session {
    /// Statut de la clé présentée par rapport au fichier `known_hosts`.
    pub fn host_key_info(&self) -> Result<HostKeyInfo, Error> {
        let path = known_hosts_path()?;
        let status = match russh::keys::check_known_hosts_path(&self.host, self.port, &self.host_key, &path) {
            Ok(true) => HostKeyStatus::Known,
            Ok(false) => HostKeyStatus::Unknown,
            Err(russh::keys::Error::KeyChanged { .. }) => HostKeyStatus::Changed,
            Err(e) => {
                return Err(Error::failed(format!(
                    "Lecture des clés d'hôte connues ({}) impossible : {e}",
                    path.display()
                )))
            }
        };
        Ok(HostKeyInfo {
            host: self.host.clone(),
            port: self.port,
            fingerprint: self.host_key.fingerprint(HashAlg::Sha256).to_string(),
            key_type: self.host_key.algorithm().as_str().to_string(),
            status,
            known_hosts: path.display().to_string(),
        })
    }

    /// Refuse une session dont la clé d'hôte n'est pas déjà connue ; appelé avant toute authentification.
    pub fn require_known_host(&self) -> Result<(), Error> {
        let info = self.host_key_info()?;
        match info.status {
            HostKeyStatus::Known => Ok(()),
            HostKeyStatus::Changed => Err(Error::refused(changed_message(&info))),
            HostKeyStatus::Unknown => Err(Error::refused(format!(
                "Machine inconnue : {}:{} présente la clé {} {}. Vérifiez cette empreinte sur la borne \
                 (ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub) puis faites-lui confiance : bouton de \
                 l'interface, ou --trust-host-key {} en ligne de commande.",
                info.host, info.port, info.key_type, info.fingerprint, info.fingerprint
            ))),
        }
    }

    /// Enregistre la clé si son empreinte est celle que l'utilisateur a confirmée ; une clé changée n'est jamais
    /// remplacée.
    pub fn trust_host_key(&self, fingerprint: &str) -> Result<HostKeyInfo, Error> {
        let info = self.host_key_info()?;
        if info.fingerprint != fingerprint.trim() {
            return Err(Error::refused(format!(
                "{}:{} présente l'empreinte {} et non {} : rien n'est enregistré.",
                info.host,
                info.port,
                info.fingerprint,
                fingerprint.trim()
            )));
        }
        match info.status {
            HostKeyStatus::Known => Ok(info),
            HostKeyStatus::Changed => Err(Error::refused(changed_message(&info))),
            HostKeyStatus::Unknown => {
                let path = PathBuf::from(&info.known_hosts);
                russh::keys::known_hosts::learn_known_hosts_path(&self.host, self.port, &self.host_key, &path)
                    .map_err(|e| Error::failed(format!("Écriture dans {} impossible : {e}", path.display())))?;
                Ok(HostKeyInfo {
                    status: HostKeyStatus::Known,
                    ..info
                })
            }
        }
    }
}

fn changed_message(info: &HostKeyInfo) -> String {
    let entry = if info.port == 22 {
        info.host.clone()
    } else {
        format!("[{}]:{}", info.host, info.port)
    };
    format!(
        "La clé d'hôte de {}:{} a changé (empreinte actuelle {}) : connexion refusée, une autre machine peut se faire \
         passer pour la borne. Si la borne a été réinstallée, retirez son ancienne entrée de {} (ssh-keygen -R {entry}) \
         puis recommencez.",
        info.host, info.port, info.fingerprint, info.known_hosts
    )
}

fn known_hosts_path() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("BOBINE_SSH_KNOWN_HOSTS") {
        return Ok(PathBuf::from(path));
    }
    home_dir()
        .map(|home| home.join(".ssh").join("known_hosts"))
        .ok_or_else(|| Error::failed("Dossier personnel introuvable : définissez BOBINE_SSH_KNOWN_HOSTS."))
}
