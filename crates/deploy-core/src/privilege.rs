use crate::orchestrate::Elevation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SudoProbe {
    pub sudo_installed: bool,
    pub user_is_sudoer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeDecision {
    UseSudo,
    BootstrapSuAsUser,
}

pub fn decide(probe: &SudoProbe) -> PrivilegeDecision {
    if probe.sudo_installed && probe.user_is_sudoer {
        PrivilegeDecision::UseSudo
    } else {
        PrivilegeDecision::BootstrapSuAsUser
    }
}

pub fn elevation_for(decision: PrivilegeDecision, login: &str) -> Elevation {
    match decision {
        PrivilegeDecision::UseSudo => Elevation::Sudo,
        PrivilegeDecision::BootstrapSuAsUser => Elevation::SuRoot {
            as_user: login.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sudoer_with_sudo_uses_sudo() {
        let d = decide(&SudoProbe {
            sudo_installed: true,
            user_is_sudoer: true,
        });
        assert_eq!(d, PrivilegeDecision::UseSudo);
        assert_eq!(elevation_for(d, "alice"), Elevation::Sudo);
    }

    #[test]
    fn missing_sudo_bootstraps_su() {
        let d = decide(&SudoProbe {
            sudo_installed: false,
            user_is_sudoer: false,
        });
        assert_eq!(d, PrivilegeDecision::BootstrapSuAsUser);
        assert_eq!(
            elevation_for(d, "alice"),
            Elevation::SuRoot {
                as_user: "alice".to_string()
            }
        );
    }

    #[test]
    fn sudo_present_but_not_sudoer_bootstraps_su() {
        let d = decide(&SudoProbe {
            sudo_installed: true,
            user_is_sudoer: false,
        });
        assert_eq!(d, PrivilegeDecision::BootstrapSuAsUser);
    }
}
