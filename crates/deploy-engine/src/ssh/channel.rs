//! Commandes distantes, téléversement et terminal.

use bobine_deploy_proto::Error;
use russh::{client, ChannelMsg};

use super::{transport, Session};

/// Sortie d'une commande d'audit : un serveur qui répond sans fin ne doit pas faire grossir la mémoire.
const MAX_OUTPUT: usize = 4 * 1024 * 1024;

impl Session {
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
}
