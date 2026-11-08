//! Moteur de Bobine Deploy : exécute une [`Command`] en émettant des [`Event`] vers un [`Sink`], puis rend un
//! [`Output`]. Les trois façades (interface graphique, ligne de commande, interface d'automatisation) n'appellent que
//! [`execute`].

pub mod discovery;
pub mod inspect;
pub mod install;
pub mod ssh;

pub use bobine_deploy_proto as proto;
use bobine_deploy_proto::{AppInfo, Command, Error, Event, Output, PROTOCOL_VERSION};

/// Destination des événements d'une commande en cours.
pub trait Sink: Send + Sync {
    fn event(&self, event: Event);
}

/// Ignore les événements (commande dont seul le résultat compte).
pub struct Silent;

impl Sink for Silent {
    fn event(&self, _: Event) {}
}

/// Version embarquée à la compilation.
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("BOBINE_VERSION").to_string(),
        commit: env!("BOBINE_COMMIT").to_string(),
        protocol: PROTOCOL_VERSION,
    }
}

/// Exécute une commande. Seul point d'entrée des façades.
pub async fn execute(command: Command, sink: &dyn Sink) -> Result<Output, Error> {
    match command {
        Command::Scan(request) => Ok(Output::Scan {
            devices: discovery::scan(sink, request.subnet).await?,
        }),
        Command::HostKey(request) => {
            let session = ssh::Session::connect(&request.host, request.port).await?;
            let info = session.host_key_info();
            session.close().await;
            Ok(Output::HostKey(info?))
        }
        Command::TrustHostKey(request) => {
            let session = ssh::Session::connect(&request.host, request.port).await?;
            let info = session.trust_host_key(&request.fingerprint);
            session.close().await;
            Ok(Output::HostKey(info?))
        }
        Command::Inspect(credentials) => Ok(Output::Inspect(inspect::inspect(&credentials).await?)),
        Command::Install(request) => Ok(Output::Install(install::install(&request, sink).await?)),
        Command::Version => Ok(Output::Version(app_info())),
    }
}
