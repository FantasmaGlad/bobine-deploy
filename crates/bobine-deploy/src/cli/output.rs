//! Affichage : JSON sur stdout avec `--json`, sinon texte lisible ; progression et journal sur stderr.

use std::io::Write;

use bobine_deploy_engine::Sink;
use bobine_deploy_proto::{Event, HostKeyStatus, Output};
use serde_json::json;

/// `println!` qui ne panique pas quand la sortie est fermée (`bobine-deploy scan | head`).
macro_rules! out {
    ($($arg:tt)*) => {{
        let _ = writeln!(std::io::stdout().lock(), $($arg)*);
    }};
}

/// Événements sur la console : JSON lignes sur stdout avec `--json`, sinon journal et progression lisibles sur stderr.
pub struct Console {
    pub json: bool,
}

impl Sink for Console {
    fn event(&self, event: Event) {
        if self.json {
            if matches!(event, Event::InstallLog(_) | Event::InstallProgress(_)) {
                out!("{}", serde_json::to_string(&event).unwrap_or_default());
            }
            return;
        }
        match event {
            Event::InstallLog(line) => eprintln!("{}", printable(&line)),
            Event::InstallProgress(p) => eprintln!("[{:>3} %] {}", p.pct, printable(&p.step_title)),
            Event::ScanProgress(p) => eprint!("\rSondage : {}/{} adresses, {} trouvée(s)", p.done, p.total, p.found),
            Event::ScanDevice(_) => {}
        }
    }
}

pub fn print_json(output: &Output) {
    match output {
        Output::Install(outcome) => out!(
            "{}",
            json!({ "event": "done", "data": { "ok": true, "access_url": outcome.access_url } })
        ),
        Output::Scan { devices } => out!("{}", serde_json::to_string_pretty(devices).unwrap_or_default()),
        Output::HostKey(info) => out!("{}", serde_json::to_string_pretty(info).unwrap_or_default()),
        Output::Inspect(report) => out!("{}", serde_json::to_string_pretty(report).unwrap_or_default()),
        Output::Version(info) => out!("{}", serde_json::to_string(info).unwrap_or_default()),
    }
}

pub fn print_human(output: &Output) {
    match output {
        Output::Version(info) => out!(
            "bobine-deploy {} ({}), contrat {}",
            info.version,
            info.commit,
            info.protocol
        ),
        Output::Scan { devices } => {
            eprintln!();
            for device in devices {
                let ports: Vec<String> = device.open_ports.iter().map(u16::to_string).collect();
                out!(
                    "{:<16} {:<28} {}{}{}",
                    device.ip,
                    printable(device.hostname.as_deref().unwrap_or("-")),
                    if device.bobine_open { "Bobine " } else { "" },
                    if device.ssh_open { "SSH " } else { "" },
                    if ports.is_empty() {
                        String::new()
                    } else {
                        format!("(ports {})", ports.join(", "))
                    },
                );
            }
            eprintln!("{} appareil(s).", devices.len());
        }
        Output::HostKey(info) => {
            let status = match info.status {
                HostKeyStatus::Known => "connue",
                HostKeyStatus::Changed => "CHANGÉE : connexion refusée",
                HostKeyStatus::Unknown => "inconnue",
            };
            out!(
                "{} {}  ({status}, {})",
                info.key_type,
                info.fingerprint,
                info.known_hosts
            );
        }
        Output::Inspect(report) => {
            out!("Système      {}", printable(&report.os_name));
            out!("Architecture {}", printable(&report.arch));
            out!("Processeur   {}", printable(&report.cpu_model));
            out!(
                "Mémoire      {} Mo, disque libre {} Go",
                report.memory_total_mb,
                report.disk_free_gb
            );
            out!("Graphismes   {}", printable(&report.gpu_info));
            out!(
                "Élévation    {}",
                if report.needs_root_password {
                    "su (mot de passe root requis)"
                } else {
                    "sudo"
                }
            );
        }
        Output::Install(outcome) => match &outcome.access_url {
            Some(url) => eprintln!("Installation terminée : {url}"),
            None => eprintln!("Installation terminée."),
        },
    }
}

/// Texte venu du réseau (nom d'hôte, journal de la cible) sans caractères de contrôle ni de direction : il ne peut ni
/// piloter le terminal (séquences ANSI, OSC) ni réordonner visuellement la ligne (U+202E…).
pub fn printable(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::printable;

    #[test]
    fn network_text_cannot_drive_the_terminal() {
        assert_eq!(printable("borne\u{1b}[31m\u{202E}"), "borne[31m");
    }
}
