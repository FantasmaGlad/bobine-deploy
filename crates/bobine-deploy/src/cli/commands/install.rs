use bobine_deploy_proto::{self as proto, Command, Error, InstallRequest};

use crate::cli::args::{Channel, Elevation, Target};
use crate::cli::input::{credentials, read_line};

pub struct Options {
    pub target: Target,
    pub channel: Channel,
    pub elevation: Elevation,
    pub no_kiosk: bool,
    pub skip_packages: bool,
    pub script: Option<String>,
    pub simulate: bool,
    pub root_password_stdin: bool,
    pub yes: bool,
}

pub async fn request(options: Options) -> Result<Command, Error> {
    if !options.yes {
        return Err(Error::usage(
            "L'installation modifie la cible : relancez avec --yes pour confirmer.",
        ));
    }
    if !options.simulate {
        super::trust(&options.target).await?;
    }
    let mut input = std::io::stdin().lock();
    let credentials = credentials(&options.target, &mut input)?;
    let root_password = if options.root_password_stdin {
        Some(read_line(&mut input, "mot de passe root")?)
    } else {
        std::env::var("BOBINE_ROOT_PASSWORD").ok().filter(|p| !p.is_empty())
    };
    Ok(Command::Install(InstallRequest {
        credentials,
        channel: match options.channel {
            Channel::Stable => proto::Channel::Stable,
            Channel::Beta => proto::Channel::Beta,
            Channel::Canary => proto::Channel::Canary,
        },
        elevation: match options.elevation {
            Elevation::Auto => proto::Elevation::Auto,
            Elevation::Sudo => proto::Elevation::Sudo,
            Elevation::Su => proto::Elevation::Su,
        },
        root_password,
        no_kiosk: options.no_kiosk,
        skip_packages: options.skip_packages,
        script_path: options.script,
        simulate: options.simulate,
    }))
}
