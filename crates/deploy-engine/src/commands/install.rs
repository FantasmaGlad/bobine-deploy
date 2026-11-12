use bobine_deploy_proto::{Error, InstallRequest, Output};

use crate::Sink;

pub async fn run(request: InstallRequest, sink: &dyn Sink) -> Result<Output, Error> {
    Ok(Output::Install(crate::install::install(&request, sink).await?))
}
