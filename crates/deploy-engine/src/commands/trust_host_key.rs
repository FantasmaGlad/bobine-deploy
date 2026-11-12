use bobine_deploy_proto::{Error, Output, TrustHostKeyRequest};

use crate::ssh::Session;

pub async fn run(request: TrustHostKeyRequest) -> Result<Output, Error> {
    let session = Session::connect(&request.host, request.port).await?;
    let info = session.trust_host_key(&request.fingerprint);
    session.close().await;
    Ok(Output::HostKey(info?))
}
