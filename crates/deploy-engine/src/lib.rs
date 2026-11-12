pub mod commands;
pub mod discovery;
pub mod inspect;
pub mod install;
pub mod ssh;

pub use bobine_deploy_proto as proto;
use bobine_deploy_proto::{Command, Error, Event, Output};
pub use commands::version::app_info;

pub trait Sink: Send + Sync {
    fn event(&self, event: Event);
}

pub struct Silent;

impl Sink for Silent {
    fn event(&self, _: Event) {}
}

pub async fn execute(command: Command, sink: &dyn Sink) -> Result<Output, Error> {
    match command {
        Command::Scan(request) => commands::scan::run(request, sink).await,
        Command::HostKey(request) => commands::host_key::run(request).await,
        Command::TrustHostKey(request) => commands::trust_host_key::run(request).await,
        Command::Inspect(credentials) => commands::inspect::run(credentials).await,
        Command::Install(request) => commands::install::run(request, sink).await,
        Command::Version => Ok(commands::version::run()),
    }
}
