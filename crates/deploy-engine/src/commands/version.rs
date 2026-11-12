//! `version` : version de Bobine Deploy et du contrat.

use bobine_deploy_proto::{AppInfo, Output, PROTOCOL_VERSION};

/// Version embarquée à la compilation.
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("BOBINE_VERSION").to_string(),
        commit: env!("BOBINE_COMMIT").to_string(),
        protocol: PROTOCOL_VERSION,
    }
}

pub fn run() -> Output {
    Output::Version(app_info())
}
