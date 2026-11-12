use std::time::Duration;

use bobine_deploy_core::progress::{Phase, ProgressState};
use bobine_deploy_proto::{Error, Event, InstallOutcome};

use super::progress::{outcome_of, relay};
use crate::Sink;

pub async fn replay(sink: &dyn Sink) -> Result<InstallOutcome, Error> {
    sink.event(Event::InstallLog(
        "> Simulation : rejeu d'une installation enregistrée, aucune machine n'est modifiée.".to_string(),
    ));
    let mut state = ProgressState::new();
    for line in include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../deploy-core/tests/fixtures/install-events.jsonl"
    ))
    .lines()
    {
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
    use std::sync::Mutex;

    use bobine_deploy_proto::{Event, InstallPhase};

    use super::*;
    use crate::install::EMBEDDED_INSTALL_SH;

    struct Collect(Mutex<Vec<Event>>);
    impl Sink for Collect {
        fn event(&self, event: Event) {
            self.0.lock().unwrap().push(event);
        }
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
        assert!(matches!(&events[0], Event::InstallLog(line) if line.contains("Simulation")));
    }
}
