//! Authentification : clé choisie, mot de passe, clavier interactif, puis agent et clés de `~/.ssh` seulement avec
//! l'accord de l'utilisateur. Une machine ne reçoit jamais d'office toutes les identités du poste.

use std::sync::Arc;

use bobine_deploy_proto::{Credentials, Error};
use russh::client::{AuthResult, KeyboardInteractiveAuthResponse};
use russh::keys::PrivateKeyWithHashAlg;
use russh::{MethodKind, MethodSet};

use super::{home_dir, transport, Session};

impl Session {
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
