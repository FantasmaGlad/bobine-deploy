//! Traduction de la sortie d'`install.sh` en événements et en résultat.

use bobine_deploy_core::progress::{classify_line, LineKind, Phase, ProgressState};
use bobine_deploy_proto::{Event, InstallOutcome, InstallPhase, InstallProgress};

use crate::Sink;

pub fn relay(line: &str, state: &mut ProgressState, sink: &dyn Sink) {
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
pub fn redact(line: &str, secret: Option<&str>) -> String {
    match secret {
        Some(secret) if !secret.is_empty() && line.contains(secret) => line.replace(secret, "***"),
        _ => line.to_string(),
    }
}

pub fn progress_of(state: &ProgressState) -> InstallProgress {
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

pub fn outcome_of(state: &ProgressState) -> InstallOutcome {
    InstallOutcome {
        access_url: state.end.as_ref().and_then(|e| e.url.clone()),
        access_port: state.end.as_ref().and_then(|e| e.port),
    }
}

#[cfg(test)]
mod tests {
    use super::redact;

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
}
