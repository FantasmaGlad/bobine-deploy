//! Traduction d'une sous-commande en commande du moteur.

use bobine_deploy_proto::{
    self as proto, Command as EngineCommand, Error, HostKeyRequest, InstallRequest, ScanRequest, TrustHostKeyRequest,
};

use super::args::{Channel, Command, Elevation, Target};
use super::input::{credentials, read_line};

pub async fn command(command: Command) -> Result<EngineCommand, Error> {
    Ok(match command {
        Command::Version => EngineCommand::Version,
        Command::Scan { subnet } => EngineCommand::Scan(ScanRequest { subnet }),
        Command::HostKey { host, port } => EngineCommand::HostKey(HostKeyRequest { host, port }),
        Command::TrustHostKey {
            host,
            port,
            fingerprint,
        } => EngineCommand::TrustHostKey(TrustHostKeyRequest {
            host,
            port,
            fingerprint,
        }),
        Command::Inspect { target } => {
            trust(&target).await?;
            let mut input = std::io::stdin().lock();
            EngineCommand::Inspect(credentials(&target, &mut input)?)
        }
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
            if !yes {
                return Err(Error::usage(
                    "L'installation modifie la cible : relancez avec --yes pour confirmer.",
                ));
            }
            if !simulate {
                trust(&target).await?;
            }
            let mut input = std::io::stdin().lock();
            let credentials = credentials(&target, &mut input)?;
            let root_password = if root_password_stdin {
                Some(read_line(&mut input, "mot de passe root")?)
            } else {
                std::env::var("BOBINE_ROOT_PASSWORD").ok().filter(|p| !p.is_empty())
            };
            EngineCommand::Install(InstallRequest {
                credentials,
                channel: match channel {
                    Channel::Stable => proto::Channel::Stable,
                    Channel::Beta => proto::Channel::Beta,
                    Channel::Canary => proto::Channel::Canary,
                },
                elevation: match elevation {
                    Elevation::Auto => proto::Elevation::Auto,
                    Elevation::Sudo => proto::Elevation::Sudo,
                    Elevation::Su => proto::Elevation::Su,
                },
                root_password,
                no_kiosk,
                skip_packages,
                script_path: script,
                simulate,
            })
        }
    })
}

/// `--trust-host-key` : enregistre la clé de la cible si son empreinte est celle donnée.
async fn trust(target: &Target) -> Result<(), Error> {
    if let Some(fingerprint) = target.trust_host_key.clone() {
        let command = EngineCommand::TrustHostKey(TrustHostKeyRequest {
            host: target.host.clone(),
            port: target.port,
            fingerprint,
        });
        bobine_deploy_engine::execute(command, &bobine_deploy_engine::Silent).await?;
    }
    Ok(())
}
