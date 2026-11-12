//! `inspect` : audit de la cible (système, architecture, élévation, matériel).

use bobine_deploy_proto::{Credentials, Error, Output};

pub async fn run(credentials: Credentials) -> Result<Output, Error> {
    Ok(Output::Inspect(crate::inspect::inspect(&credentials).await?))
}
