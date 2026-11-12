mod host_key;
mod inspect;
mod install;
mod scan;
mod trust_host_key;

use bobine_deploy_proto::{Command as EngineCommand, Error};

use super::args::{Command, Target};

pub async fn command(command: Command) -> Result<EngineCommand, Error> {
    Ok(match command {
        Command::Version => EngineCommand::Version,
        Command::Scan { subnet } => scan::request(subnet),
        Command::HostKey { host, port } => host_key::request(host, port),
        Command::TrustHostKey {
            host,
            port,
            fingerprint,
        } => trust_host_key::request(host, port, fingerprint),
        Command::Inspect { target } => inspect::request(&target).await?,
        Command::Install {
            target,
            channel,
            elevation,
            no_kiosk,
            skip_packages,
            script,
            simulate,
            root_password_stdin,
            yes,
        } => {
            install::request(install::Options {
                target,
                channel,
                elevation,
                no_kiosk,
                skip_packages,
                script,
                simulate,
                root_password_stdin,
                yes,
            })
            .await?
        }
    })
}

async fn trust(target: &Target) -> Result<(), Error> {
    if let Some(fingerprint) = target.trust_host_key.clone() {
        let command = trust_host_key::request(target.host.clone(), target.port, fingerprint);
        bobine_deploy_engine::execute(command, &bobine_deploy_engine::Silent).await?;
    }
    Ok(())
}
