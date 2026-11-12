use bobine_deploy_core::privilege::{decide, PrivilegeDecision, SudoProbe};
use bobine_deploy_proto::{Elevation, Error};

use crate::ssh::Session;

pub async fn sudo_probe(session: &Session) -> Result<SudoProbe, Error> {
    let yes = |out: String| out.trim() == "yes";
    let sudo_installed = yes(session
        .exec("command -v sudo >/dev/null 2>&1 && echo yes || echo no")
        .await?
        .0);
    let user_is_sudoer = sudo_installed
        && yes(session
            .exec("sudo -n true >/dev/null 2>&1 && echo yes || echo no")
            .await?
            .0);
    Ok(SudoProbe {
        sudo_installed,
        user_is_sudoer,
    })
}

pub async fn probe(session: &Session) -> Result<Elevation, Error> {
    Ok(match decide(&sudo_probe(session).await?) {
        PrivilegeDecision::UseSudo => Elevation::Sudo,
        PrivilegeDecision::BootstrapSuAsUser => Elevation::Su,
    })
}
