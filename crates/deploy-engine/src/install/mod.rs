//! Installation de Bobine : téléverse l'`install.sh` embarqué (release signée de Bobine, épinglée dans
//! `bobine-release.toml`), le lance avec l'élévation choisie et traduit sa sortie `--progress=json` en événements.
//! `install.sh` reste la seule source de vérité de l'installation.

mod elevation;
mod progress;
mod replay;
mod stream;

use bobine_deploy_core::orchestrate::{
    build_remote_command, build_remote_command_via, shell_quote, Elevation as CoreElevation, InstallOptions,
};
use bobine_deploy_core::progress::Phase;
use bobine_deploy_proto::{Elevation, Error, Event, InstallOutcome, InstallRequest};
use zeroize::Zeroizing;

use crate::ssh::{self, Session};
use crate::Sink;

#[doc(hidden)]
pub use stream::stream;

/// `install.sh` de la release de Bobine épinglée, vérifié à la compilation (`build.rs`).
pub const EMBEDDED_INSTALL_SH: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor/install.sh"));

pub async fn install(request: &InstallRequest, sink: &dyn Sink) -> Result<InstallOutcome, Error> {
    if request.simulate {
        return replay::replay(sink).await;
    }
    let session = ssh::open(&request.credentials).await?;
    let result = run(&session, request, sink).await;
    session.close().await;
    result
}

async fn run(session: &Session, request: &InstallRequest, sink: &dyn Sink) -> Result<InstallOutcome, Error> {
    let elevation = match request.elevation {
        Elevation::Auto => elevation::probe(session).await?,
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

    let outcome = stream::stream(session, &command, root_password.as_deref(), sink).await;

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
    Ok(progress::outcome_of(&state))
}

fn remote_script_path() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("/tmp/bobine-install-{}-{nanos}.sh", std::process::id())
}
