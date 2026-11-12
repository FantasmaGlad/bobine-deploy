use bobine_deploy_proto::{Command, Error};

use crate::cli::args::Target;
use crate::cli::input::credentials;

pub async fn request(target: &Target) -> Result<Command, Error> {
    super::trust(target).await?;
    let mut input = std::io::stdin().lock();
    Ok(Command::Inspect(credentials(target, &mut input)?))
}
