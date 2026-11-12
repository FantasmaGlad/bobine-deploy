//! `host-key` : empreinte et statut de la clé d'hôte SSH, sans s'authentifier.

use bobine_deploy_proto::{Error, HostKeyRequest, Output};

use crate::ssh::Session;

pub async fn run(request: HostKeyRequest) -> Result<Output, Error> {
    let session = Session::connect(&request.host, request.port).await?;
    let info = session.host_key_info();
    session.close().await;
    Ok(Output::HostKey(info?))
}
