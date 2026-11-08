//! Élévation déterminée comme l'audit : `sudo` si l'utilisateur peut s'en servir sans mot de passe, sinon `su -`.

use bobine_deploy_core::privilege::{decide, PrivilegeDecision, SudoProbe};
use bobine_deploy_proto::{Elevation, Error};

use crate::ssh::Session;

pub async fn probe(session: &Session) -> Result<Elevation, Error> {
    let sudo_installed = session
        .exec("command -v sudo >/dev/null 2>&1 && echo yes || echo no")
        .await?
        .0
        == "yes";
    let user_is_sudoer = sudo_installed
        && session
            .exec("sudo -n true >/dev/null 2>&1 && echo yes || echo no")
            .await?
            .0
            == "yes";
    Ok(
        match decide(&SudoProbe {
            sudo_installed,
            user_is_sudoer,
        }) {
            PrivilegeDecision::UseSudo => Elevation::Sudo,
            PrivilegeDecision::BootstrapSuAsUser => Elevation::Su,
        },
    )
}
