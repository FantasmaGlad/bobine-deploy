use bobine_deploy_proto::{Error, Output, ScanRequest};

use crate::{discovery, Sink};

pub async fn run(request: ScanRequest, sink: &dyn Sink) -> Result<Output, Error> {
    Ok(Output::Scan {
        devices: discovery::scan(sink, request.subnet).await?,
    })
}
