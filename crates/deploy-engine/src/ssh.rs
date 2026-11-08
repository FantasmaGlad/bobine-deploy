//! SSH en Rust pur (russh) : connexion, vérification de la clé d'hôte avant tout secret, authentification, commandes
//! distantes et téléversement.
//!
//! Clé d'hôte : `~/.ssh/known_hosts` (ou `BOBINE_SSH_KNOWN_HOSTS`), au format d'OpenSSH, entrées hachées comprises.
//! Une clé connue passe ; une clé inconnue n'est enregistrée qu'après confirmation de son empreinte ; une clé qui a
//! changé est toujours refusée. Sur un Wi-Fi partagé, une machine qui usurpe l'adresse de la borne recevrait sinon le
//! mot de passe SSH, voire celui de root.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bobine_deploy_proto::{Credentials, Error, HostKeyInfo, HostKeyStatus};
use russh::client::{self, AuthResult, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::ssh_key::{HashAlg, PublicKey};
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{ChannelMsg, MethodKind, MethodSet};
use tokio::net::TcpStream;
use tokio::time::timeout;

const TCP_TIMEOUT: Duration = Duration::from_secs(8);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
/// Une installation reste parfois silencieuse plusieurs minutes (compilation) : sans messages de maintien, une box ou
/// un Wi-Fi qui purge les connexions inactives la couperait.
const KEEPALIVE: Duration = Duration::from_secs(15);
/// Sortie d'une commande d'audit : un serveur qui répond sans fin ne doit pas faire grossir la mémoire.
const MAX_OUTPUT: usize = 4 * 1024 * 1024;

/// Retient la clé présentée pendant la négociation ; la décision de confiance est prise ensuite, avant tout secret.
struct Recorder {
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

/// Session SSH négociée, pas encore authentifiée tant que [`Session::authenticate`] n'a pas réussi.
pub struct Session {
    handle: Handle<Recorder>,
    host: String,
    port: u16,
    host_key: PublicKey,
}

impl Session {
    /// Connexion TCP (IPv4 d'abord : le scan n'énumère que de l'IPv4) et négociation SSH, sans authentification.
    pub async fn connect(host: &str, port: u16) -> Result<Self, Error> {
        let host = host.trim();
        if host.is_empty() {
            return Err(Error::usage(
                "Adresse de la borne manquante : sélectionnez-la dans le scan ou saisissez son IP.",
            ));
        }
        let mut addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| {
                Error::unreachable(format!(
                    "Nom ou adresse « {host} » introuvable ({e}). Essayez l'adresse IP de la borne (visible dans \
                     l'interface du routeur)."
                ))
            })?
            .collect();
        addrs.sort_by_key(SocketAddr::is_ipv6);
        if addrs.is_empty() {
            return Err(Error::unreachable(format!("Aucune adresse trouvée pour « {host} ».")));
        }

        let mut last_error = String::new();
        let mut stream = None;
        for addr in &addrs {
            match timeout(TCP_TIMEOUT, TcpStream::connect(addr)).await {
                Ok(Ok(tcp)) => {
                    stream = Some(tcp);
                    break;
                }
                Ok(Err(e)) => last_error = describe_connect_error(host, port, &e),
                Err(_) => last_error = describe_timeout(host, port),
            }
        }
        let stream = stream.ok_or_else(|| Error::unreachable(last_error))?;
        stream.set_nodelay(true).ok();

        let config = Arc::new(client::Config {
            keepalive_interval: Some(KEEPALIVE),
            keepalive_max: 8,
            nodelay: true,
            ..Default::default()
        });
        let seen = Arc::new(Mutex::new(None));
        let recorder = Recorder { key: seen.clone() };
        let handle = timeout(HANDSHAKE_TIMEOUT, client::connect_stream(config, stream, recorder))
            .await
            .map_err(|_| Error::unreachable(format!("{host}:{port} ne termine pas la négociation SSH.")))?
            .map_err(|e| {
                Error::unreachable(format!(
                    "Échec de la négociation SSH avec {host}:{port} ({e}) : ce port héberge-t-il bien un serveur SSH ?"
                ))
            })?;
        let host_key = seen
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .ok_or_else(|| Error::failed(format!("{host}:{port} n'a présenté aucune clé d'hôte SSH.")))?;
        Ok(Self {
            handle,
            host: host.to_string(),
            port,
            host_key,
        })
    }

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

    /// Authentifie la session, dans l'ordre : clé choisie explicitement (avec le mot de passe comme phrase de passe
    /// si elle est chiffrée), mot de passe, clavier interactif (PAM), puis, seulement avec l'accord de l'utilisateur
    /// (`local_keys`), l'agent SSH et les clés usuelles de `~/.ssh`. Une machine ne reçoit jamais d'office toutes les
    /// identités du poste.
    pub async fn authenticate(&mut self, creds: &Credentials) -> Result<(), Error> {
        let user = creds.username.trim().to_string();
        if user.is_empty() {
            return Err(Error::usage("Utilisateur SSH manquant."));
        }
        let password = creds.password.as_deref().filter(|p| !p.trim().is_empty());
        let mut methods: Option<MethodSet> = None;

        if let Some(path) = creds.key_path.as_deref().filter(|p| !p.trim().is_empty()) {
            let key = russh::keys::load_secret_key(path, None)
                .or_else(|first| match password {
                    Some(passphrase) => russh::keys::load_secret_key(path, Some(passphrase)),
                    None => Err(first),
                })
                .map_err(|e| Error::refused(format!("Clé privée {path} illisible ou phrase de passe requise : {e}")))?;
            if self.try_private_key(&user, key, &mut methods).await? {
                return Ok(());
            }
        }

        if let Some(password) = password {
            let result = self
                .handle
                .authenticate_password(user.clone(), password)
                .await
                .map_err(transport)?;
            if record(result, &mut methods) {
                return Ok(());
            }
            if self.keyboard_interactive(&user, password).await? {
                return Ok(());
            }
        }

        if creds.local_keys {
            if self.try_agent(&user, &mut methods).await? {
                return Ok(());
            }
            if let Some(dir) = home_dir().map(|home| home.join(".ssh")) {
                for name in ["id_ed25519", "id_ecdsa", "id_rsa"] {
                    let path = dir.join(name);
                    // Une clé chiffrée sans phrase de passe connue est ignorée : elle n'est pas le choix explicite.
                    if let Ok(key) = russh::keys::load_secret_key(&path, None) {
                        if self.try_private_key(&user, key, &mut methods).await? {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(Error::refused(refusal_message(
            &user,
            password.is_some(),
            methods.as_ref(),
        )))
    }

    async fn try_private_key(
        &mut self,
        user: &str,
        key: russh::keys::PrivateKey,
        methods: &mut Option<MethodSet>,
    ) -> Result<bool, Error> {
        let hash = self
            .handle
            .best_supported_rsa_hash()
            .await
            .map_err(transport)?
            .flatten();
        let result = self
            .handle
            .authenticate_publickey(user.to_string(), PrivateKeyWithHashAlg::new(Arc::new(key), hash))
            .await
            .map_err(transport)?;
        Ok(record(result, methods))
    }

    async fn keyboard_interactive(&mut self, user: &str, password: &str) -> Result<bool, Error> {
        let mut response = self
            .handle
            .authenticate_keyboard_interactive_start(user.to_string(), None::<String>)
            .await
            .map_err(transport)?;
        // Quelques tours au plus : chaque invite reçoit le mot de passe (PAM classique : une seule invite).
        for _ in 0..4 {
            match response {
                KeyboardInteractiveAuthResponse::Success => return Ok(true),
                KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
                KeyboardInteractiveAuthResponse::InfoRequest { ref prompts, .. } => {
                    let answers = prompts.iter().map(|_| password.to_string()).collect();
                    response = self
                        .handle
                        .authenticate_keyboard_interactive_respond(answers)
                        .await
                        .map_err(transport)?;
                }
            }
        }
        Ok(false)
    }

    async fn try_agent(&mut self, user: &str, methods: &mut Option<MethodSet>) -> Result<bool, Error> {
        #[cfg(unix)]
        let agent = russh::keys::agent::client::AgentClient::connect_env().await.ok();
        #[cfg(windows)]
        let agent = russh::keys::agent::client::AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent")
            .await
            .ok();
        let Some(mut agent) = agent else {
            return Ok(false);
        };
        let identities = agent.request_identities().await.unwrap_or_default();
        let hash = self
            .handle
            .best_supported_rsa_hash()
            .await
            .map_err(transport)?
            .flatten();
        for identity in identities {
            let russh::keys::agent::AgentIdentity::PublicKey { key, .. } = identity else {
                continue;
            };
            let result = self
                .handle
                .authenticate_publickey_with(user.to_string(), key, hash, &mut agent)
                .await;
            if let Ok(result) = result {
                if record(result, methods) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Commande d'audit : sortie standard (bornée) et code de sortie.
    pub async fn exec(&self, command: &str) -> Result<(String, Option<u32>), Error> {
        let mut channel = self.handle.channel_open_session().await.map_err(transport)?;
        channel
            .exec(true, command)
            .await
            .map_err(|e| Error::failed(format!("Exécution de « {command} » impossible : {e}")))?;
        let mut output = Vec::new();
        let mut code = None;
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => {
                    let room = MAX_OUTPUT.saturating_sub(output.len());
                    output.extend_from_slice(&data[..data.len().min(room)]);
                }
                ChannelMsg::ExitStatus { exit_status } => code = Some(exit_status),
                _ => {}
            }
        }
        Ok((String::from_utf8_lossy(&output).trim().to_string(), code))
    }

    /// Écrit `bytes` dans `remote_path` sur la cible, lisible du seul propriétaire (0700), par `cat` sur l'entrée
    /// standard : aucun sous-système SFTP n'est requis.
    pub async fn upload(&self, remote_path: &str, bytes: &[u8]) -> Result<(), Error> {
        let quoted = bobine_deploy_core::orchestrate::shell_quote(remote_path);
        let command = format!("umask 077 && cat > {quoted} && chmod 700 {quoted}");
        let mut channel = self.handle.channel_open_session().await.map_err(transport)?;
        channel
            .exec(true, command)
            .await
            .map_err(|e| Error::failed(format!("Téléversement de l'installateur impossible : {e}")))?;
        channel
            .data(bytes)
            .await
            .map_err(|e| Error::failed(format!("Envoi de l'installateur interrompu : {e}")))?;
        channel.eof().await.map_err(transport)?;
        let mut code = None;
        while let Some(message) = channel.wait().await {
            if let ChannelMsg::ExitStatus { exit_status } = message {
                code = Some(exit_status);
            }
        }
        match code {
            Some(0) => Ok(()),
            other => Err(Error::failed(format!(
                "Impossible d'écrire l'installateur sur la cible ({remote_path}, code {other:?})."
            ))),
        }
    }

    /// Canal avec terminal, écho coupé : `su -` (et certains `sudo`) ne présentent leur invite de mot de passe que
    /// si un terminal est alloué, et un mot de passe écrit avec l'écho actif reviendrait dans le journal.
    pub async fn open_terminal(&self, command: &str) -> Result<russh::Channel<client::Msg>, Error> {
        let channel = self.handle.channel_open_session().await.map_err(transport)?;
        channel
            .request_pty(false, "xterm", 200, 50, 0, 0, &[(russh::Pty::ECHO, 0)])
            .await
            .map_err(transport)?;
        channel
            .exec(true, command)
            .await
            .map_err(|e| Error::failed(format!("Lancement de la commande impossible : {e}")))?;
        Ok(channel)
    }

    pub async fn close(self) {
        let _ = self.handle.disconnect(russh::Disconnect::ByApplication, "", "fr").await;
    }
}

/// Connexion, clé d'hôte déjà connue, puis authentification : la seule porte d'entrée vers une session utilisable.
pub async fn open(creds: &Credentials) -> Result<Session, Error> {
    let mut session = Session::connect(&creds.host, creds.port).await?;
    session.require_known_host()?;
    session.authenticate(creds).await?;
    Ok(session)
}

fn record(result: AuthResult, methods: &mut Option<MethodSet>) -> bool {
    match result {
        AuthResult::Success => true,
        AuthResult::Failure { remaining_methods, .. } => {
            *methods = Some(remaining_methods);
            false
        }
    }
}

fn refusal_message(user: &str, had_password: bool, methods: Option<&MethodSet>) -> String {
    let names: Vec<&str> = methods
        .map(|set| set.iter().map(<&str>::from).collect())
        .unwrap_or_default();
    let allows_password = methods.is_some_and(|set| {
        set.iter()
            .any(|m| matches!(m, MethodKind::Password | MethodKind::KeyboardInteractive))
    });
    let list = names.join(", ");
    if methods.is_some() && !allows_password {
        format!(
            "Le serveur SSH de la cible refuse les mots de passe (PasswordAuthentication désactivé). Méthodes \
             acceptées : [{list}]. Activez « PasswordAuthentication yes » sur la cible ou installez votre clé publique \
             dans ~/.ssh/authorized_keys."
        )
    } else if had_password {
        format!("Authentification refusée pour « {user} » : mot de passe incorrect ou clés refusées.")
    } else {
        format!(
            "Aucun mot de passe ni clé acceptés par le serveur. Méthodes acceptées : [{list}]. Choisissez une clé, ou \
             autorisez l'agent SSH et les clés de ~/.ssh."
        )
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

fn describe_connect_error(host: &str, port: u16, e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::ConnectionRefused => format!(
            "Connexion refusée par {host}:{port} : la machine est joignable mais SSH n'y est pas actif. Sur la borne : \
             « sudo apt install openssh-server » puis « sudo systemctl enable --now ssh »."
        ),
        std::io::ErrorKind::TimedOut => describe_timeout(host, port),
        _ => format!("Impossible de joindre {host}:{port} : {e}"),
    }
}

fn describe_timeout(host: &str, port: u16) -> String {
    format!(
        "Délai dépassé vers {host}:{port} : machine éteinte, adresse erronée, autre réseau, ou isolation des clients \
         activée sur le Wi-Fi (réglage « isolation AP » ou réseau invité du routeur)."
    )
}

fn transport(e: russh::Error) -> Error {
    Error::failed(format!("Erreur SSH : {e}"))
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn known_hosts_path() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("BOBINE_SSH_KNOWN_HOSTS") {
        return Ok(PathBuf::from(path));
    }
    home_dir()
        .map(|home| home.join(".ssh").join("known_hosts"))
        .ok_or_else(|| Error::failed("Dossier personnel introuvable : définissez BOBINE_SSH_KNOWN_HOSTS."))
}
