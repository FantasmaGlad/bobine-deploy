//! Lecture du terminal d'installation : invite du mot de passe root, lignes de progression et de journal.

use std::time::{Duration, Instant};

use bobine_deploy_core::progress::ProgressState;
use bobine_deploy_proto::{Error, Event};
use russh::ChannelMsg;
use zeroize::Zeroize;

use super::progress::{redact, relay};
use crate::ssh::Session;
use crate::Sink;

/// Délai pour voir l'invite de `su -` : au-delà, le mot de passe ne peut pas être transmis.
const PROMPT_DEADLINE: Duration = Duration::from_secs(20);
/// Une cible qui n'envoie jamais de saut de ligne ne doit pas faire croître la mémoire sans fin.
const MAX_PENDING_BYTES: usize = 1 << 20;

/// Exécute la commande sur un terminal, transmet le mot de passe root à l'invite de `su -`, puis relaie chaque ligne
/// (événement de progression ou journal). Rend l'état final et le code de sortie.
pub async fn stream(
    session: &Session,
    command: &str,
    root_password: Option<&str>,
    sink: &dyn Sink,
) -> Result<(ProgressState, Option<u32>), Error> {
    let mut channel = session.open_terminal(command).await?;
    let mut state = ProgressState::new();
    let mut pending: Vec<u8> = Vec::new();
    let mut password_sent = root_password.is_none();
    let deadline = Instant::now() + PROMPT_DEADLINE;
    let mut exit_code = None;

    loop {
        let message =
            if password_sent {
                channel.wait().await
            } else {
                let left = deadline.saturating_duration_since(Instant::now());
                match tokio::time::timeout(left, channel.wait()).await {
                    Ok(message) => message,
                    Err(_) => return Err(Error::failed(
                        "Le mot de passe root n'a pas pu être transmis : invite de mot de passe non détectée sur la \
                         cible.",
                    )),
                }
            };
        let Some(message) = message else { break };
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                pending.extend_from_slice(&data);
                if pending.len() > MAX_PENDING_BYTES && !pending.contains(&b'\n') {
                    pending.clear();
                    sink.event(Event::InstallLog(
                        "[ATTENTION] Sortie distante sans fin de ligne ignorée".to_string(),
                    ));
                }
                // L'invite n'est jamais suivie d'un saut de ligne : elle se lit sur le tampon brut.
                if !password_sent {
                    let lower = String::from_utf8_lossy(&pending).to_lowercase();
                    if lower.contains("password") || lower.contains("mot de passe") {
                        if let Some(password) = root_password {
                            let mut line = format!("{password}\n");
                            let sent = channel.data_bytes(line.clone().into_bytes()).await;
                            line.zeroize();
                            sent.map_err(|e| {
                                Error::failed(format!("Transmission du mot de passe root impossible : {e}"))
                            })?;
                        }
                        password_sent = true;
                        pending.clear();
                    }
                }
                while let Some(position) = pending.iter().position(|&b| b == b'\n') {
                    let raw: Vec<u8> = pending.drain(..=position).collect();
                    let line = String::from_utf8_lossy(&raw);
                    let line = redact(line.trim_end_matches(['\r', '\n']), root_password);
                    relay(&line, &mut state, sink);
                }
            }
            ChannelMsg::ExitStatus { exit_status } => exit_code = Some(exit_status),
            _ => {}
        }
    }
    Ok((state, exit_code))
}
