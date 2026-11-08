//! Installation de Bobine sur la cible : téléverse l'`install.sh` embarqué (release signée de Bobine, épinglée dans
//! `bobine-release.toml`), le lance avec l'élévation choisie sur un terminal SSH et traduit sa sortie
//! `--progress=json` en événements. `install.sh` reste la seule source de vérité de l'installation.

use std::time::{Duration, Instant};

use bobine_deploy_core::orchestrate::{
    build_remote_command, build_remote_command_via, shell_quote, Elevation as CoreElevation, InstallOptions,
};
use bobine_deploy_core::privilege::{decide, PrivilegeDecision, SudoProbe};
use bobine_deploy_core::progress::{classify_line, LineKind, Phase, ProgressState};
use bobine_deploy_proto::{Elevation, Error, Event, InstallOutcome, InstallPhase, InstallProgress, InstallRequest};
use russh::ChannelMsg;
use zeroize::{Zeroize, Zeroizing};

use crate::ssh::{self, Session};
use crate::Sink;

/// `install.sh` de la release de Bobine épinglée, vérifié à la compilation (`build.rs`).
pub const EMBEDDED_INSTALL_SH: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor/install.sh"));

/// Délai pour voir l'invite de mot de passe de `su -` : au-delà, le mot de passe ne peut pas être transmis.
const PROMPT_DEADLINE: Duration = Duration::from_secs(20);
/// Une cible qui n'envoie jamais de saut de ligne ne doit pas faire croître la mémoire sans fin.
const MAX_PENDING_BYTES: usize = 1 << 20;

pub async fn install(request: &InstallRequest, sink: &dyn Sink) -> Result<InstallOutcome, Error> {
    if request.simulate {
        return replay(sink).await;
    }
    let session = ssh::open(&request.credentials).await?;
    let result = run(&session, request, sink).await;
    session.close().await;
    result
}

async fn run(session: &Session, request: &InstallRequest, sink: &dyn Sink) -> Result<InstallOutcome, Error> {
    let elevation = match request.elevation {
        Elevation::Auto => probe_elevation(session).await?,
        other => other,
    };
    let root_password = Zeroizing::new(request.root_password.clone().filter(|p| !p.trim().is_empty()));
    let needs_root_password = elevation == Elevation::Su;
    if needs_root_password && root_password.is_none() {
        return Err(Error::usage(
            "Le mot de passe root est requis pour installer via « su - » sur cette machine, mais aucun n'a été fourni.",
        ));
    }

    let (script, uploaded) = match request.script_path.clone() {
        Some(path) => (path, false),
        None => {
            let remote = remote_script_path();
            sink.event(Event::InstallLog("> Envoi de l'installateur sur la cible…".to_string()));
            session.upload(&remote, EMBEDDED_INSTALL_SH.as_bytes()).await?;
            (remote, true)
        }
    };

    let core_elevation = if needs_root_password {
        CoreElevation::SuRoot {
            as_user: request.credentials.username.clone(),
        }
    } else {
        CoreElevation::Sudo
    };
    let options = InstallOptions {
        no_kiosk: request.no_kiosk,
        skip_packages: request.skip_packages,
        progress_json: true,
        channel: request.channel.as_str().to_string(),
        ..Default::default()
    };
    // /tmp peut être monté noexec : le script téléversé passe par bash.
    let command = if uploaded {
        build_remote_command_via("bash", &script, &options, &core_elevation)
    } else {
        build_remote_command(&script, &options, &core_elevation)
    };
    sink.event(Event::InstallLog(format!("> Exécution distante : {command}")));

    let outcome = stream(session, &command, root_password.as_deref(), sink).await;

    if uploaded {
        let _ = session.exec(&format!("rm -f {}", shell_quote(&script))).await;
    }
    let (state, exit_code) = outcome?;
    // Le code de sortie fait foi : un `run_end` de succès ne masque pas un échec réel (script tué, sudo refusé,
    // sortie falsifiée par la cible), et un `run_end` absent est aussi un échec.
    match exit_code {
        Some(0) => {}
        code => {
            return Err(Error::failed(format!(
                "L'installation s'est interrompue (code de sortie {}). Consultez le journal et /var/log/bobine/ \
                 sur la borne.",
                code.map_or_else(|| "inconnu".to_string(), |c| c.to_string())
            )))
        }
    }
    if state.phase != Phase::Succeeded {
        return Err(Error::failed(
            "L'installation s'est terminée sans annoncer sa réussite (run_end). Consultez le journal.",
        ));
    }
    Ok(outcome_of(&state))
}

/// Exécute la commande sur un terminal, transmet le mot de passe root à l'invite de `su -`, puis relaie chaque ligne
/// (événement de progression ou journal). Rend l'état final et le code de sortie.
#[doc(hidden)]
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

/// Élévation déterminée comme l'audit : `sudo` si l'utilisateur peut s'en servir sans mot de passe, sinon `su -`.
async fn probe_elevation(session: &Session) -> Result<Elevation, Error> {
    let sudo_installed = session
        .exec("command -v sudo >/dev/null 2>&1 && echo yes || echo no")
        .await?
        .0
        == "yes";
    let user_is_sudoer = sudo_installed
        && session
            .exec("sudo -n true >/dev/null 2>&1 && echo yes || echo no")
            .await?
            .0
            == "yes";
    Ok(
        match decide(&SudoProbe {
            sudo_installed,
            user_is_sudoer,
        }) {
            PrivilegeDecision::UseSudo => Elevation::Sudo,
            PrivilegeDecision::BootstrapSuAsUser => Elevation::Su,
        },
    )
}

fn relay(line: &str, state: &mut ProgressState, sink: &dyn Sink) {
    match classify_line(line) {
        LineKind::Event(event) => {
            state.apply(&event);
            sink.event(Event::InstallProgress(progress_of(state)));
        }
        LineKind::Log => sink.event(Event::InstallLog(line.to_string())),
        LineKind::Malformed(error) => {
            sink.event(Event::InstallLog(format!("[ATTENTION] Évènement malformé : {error}")))
        }
    }
}

/// Le mot de passe root ne doit jamais apparaître dans un journal affiché ou copié.
pub(crate) fn redact(line: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() && line.contains(secret) => line.replace(secret, "***"),
        _ => line.to_string(),
    }
}

pub(crate) fn progress_of(state: &ProgressState) -> InstallProgress {
    let (step_index, step_title, step_slug) = match &state.current {
        Some(step) => (step.index, step.title.clone(), step.slug.clone()),
        None => (0, "Initialisation…".to_string(), "init".to_string()),
    };
    InstallProgress {
        pct: state.pct,
        step_index,
        total_steps: state.total,
        step_title,
        step_slug,
        phase: match state.phase {
            Phase::NotStarted => InstallPhase::NotStarted,
            Phase::Running => InstallPhase::Running,
            Phase::Succeeded => InstallPhase::Succeeded,
            Phase::Failed => InstallPhase::Failed,
        },
        error_message: state
            .failure
            .as_ref()
            .map(|f| format!("Échec à l'étape {} ({}) : {}", f.index, f.slug, f.title)),
        access_url: state.end.as_ref().and_then(|e| e.url.clone()),
        access_port: state.end.as_ref().and_then(|e| e.port),
    }
}

fn outcome_of(state: &ProgressState) -> InstallOutcome {
    InstallOutcome {
        access_url: state.end.as_ref().and_then(|e| e.url.clone()),
        access_port: state.end.as_ref().and_then(|e| e.port),
    }
}

fn remote_script_path() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("/tmp/bobine-install-{}-{nanos}.sh", std::process::id())
}

/// Rejoue une installation réelle enregistrée (fixture du contrat `--progress=json`), sans toucher de cible.
async fn replay(sink: &dyn Sink) -> Result<InstallOutcome, Error> {
    sink.event(Event::InstallLog(
        "> Simulation : rejeu d'une installation enregistrée, aucune machine n'est modifiée.".to_string(),
    ));
    let mut state = ProgressState::new();
    for line in include_str!("../../deploy-core/tests/fixtures/install-events.jsonl").lines() {
        tokio::time::sleep(Duration::from_millis(120)).await;
        relay(line, &mut state, sink);
    }
    if state.phase != Phase::Succeeded {
        return Err(Error::failed("La simulation ne s'est pas terminée par une réussite."));
    }
    Ok(outcome_of(&state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Collect(Mutex<Vec<Event>>);
    impl Sink for Collect {
        fn event(&self, event: Event) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[test]
    fn the_root_password_never_reaches_the_log() {
        assert_eq!(
            redact("mot de passe: hunter2 ok", Some("hunter2")),
            "mot de passe: *** ok"
        );
        assert_eq!(redact("rien", Some("hunter2")), "rien");
        assert_eq!(redact("rien", Some("")), "rien");
        assert_eq!(redact("rien", None), "rien");
    }

    #[test]
    fn the_embedded_installer_speaks_the_progress_contract() {
        assert!(EMBEDDED_INSTALL_SH.starts_with("#!"));
        assert!(EMBEDDED_INSTALL_SH.contains("--progress=json"));
    }

    #[tokio::test(start_paused = true)]
    async fn the_simulation_replays_a_real_installation_to_its_end() {
        let sink = Collect(Mutex::new(Vec::new()));
        let outcome = replay(&sink).await.unwrap();
        let events = sink.0.into_inner().unwrap();
        let last = events
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::InstallProgress(p) => Some(p.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(last.phase, InstallPhase::Succeeded);
        assert_eq!(last.pct, 100);
        assert_eq!(outcome.access_url, last.access_url);
        assert!(events.iter().any(|e| matches!(e, Event::InstallLog(_))));
    }
}
