#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallOptions {
    pub no_kiosk: bool,
    pub skip_packages: bool,
    pub skip_build: bool,
    pub dry_run: bool,
    pub progress_json: bool,
    pub channel: String,
}

impl InstallOptions {
    pub fn assistant_default() -> Self {
        InstallOptions {
            progress_json: true,
            channel: "stable".to_string(),
            ..Default::default()
        }
    }

    pub fn to_args(&self) -> Vec<String> {
        let mut args = vec!["-y".to_string()];
        if self.progress_json {
            args.push("--progress=json".to_string());
        }
        if !self.channel.is_empty() {
            args.push(format!("--channel={}", self.channel));
        }
        if self.no_kiosk {
            args.push("--no-kiosk".to_string());
        }
        if self.skip_packages {
            args.push("--skip-packages".to_string());
        }
        if self.skip_build {
            args.push("--skip-build".to_string());
        }
        if self.dry_run {
            args.push("--dry-run".to_string());
        }
        args
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elevation {
    Sudo,
    SuRoot { as_user: String },
}

fn is_shell_safe(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '=' | ':' | ',' | '+' | '@' | '%')
        })
}

pub fn shell_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

pub fn shell_quote(token: &str) -> String {
    if is_shell_safe(token) {
        token.to_string()
    } else {
        shell_single_quote(token)
    }
}

pub fn build_remote_command(script_path: &str, opts: &InstallOptions, elev: &Elevation) -> String {
    build_command(None, script_path, opts, elev)
}

pub fn build_remote_command_via(
    interpreter: &str,
    script_path: &str,
    opts: &InstallOptions,
    elev: &Elevation,
) -> String {
    build_command(Some(interpreter), script_path, opts, elev)
}

fn build_command(interpreter: Option<&str>, script_path: &str, opts: &InstallOptions, elev: &Elevation) -> String {
    match elev {
        Elevation::Sudo => {
            let mut parts = vec!["sudo".to_string()];
            if let Some(i) = interpreter {
                parts.push(shell_quote(i));
            }
            parts.push(shell_quote(script_path));
            parts.extend(opts.to_args().iter().map(|a| shell_quote(a)));
            parts.join(" ")
        }
        Elevation::SuRoot { as_user } => {
            let mut inner = Vec::new();
            if let Some(i) = interpreter {
                inner.push(shell_quote(i));
            }
            inner.push(shell_quote(script_path));
            inner.push("--as-user".to_string());
            inner.push(shell_quote(as_user));
            inner.extend(opts.to_args().iter().map(|a| shell_quote(a)));
            let inner_cmd = inner.join(" ");
            format!("su - -c {}", shell_single_quote(&inner_cmd))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_default_is_noninteractive_json() {
        let a = InstallOptions::assistant_default().to_args();
        assert_eq!(a, vec!["-y", "--progress=json", "--channel=stable"]);
    }

    #[test]
    fn args_reflect_flags_in_stable_order() {
        let opts = InstallOptions {
            no_kiosk: true,
            skip_packages: true,
            skip_build: false,
            dry_run: true,
            progress_json: true,
            channel: String::new(),
        };
        assert_eq!(
            opts.to_args(),
            vec!["-y", "--progress=json", "--no-kiosk", "--skip-packages", "--dry-run"]
        );
    }

    #[test]
    fn channel_flag_is_emitted_when_set() {
        let opts = InstallOptions {
            channel: "beta".to_string(),
            ..Default::default()
        };
        assert_eq!(opts.to_args(), vec!["-y", "--channel=beta"]);
    }

    #[test]
    fn sudo_command_is_readable_when_safe() {
        let cmd = build_remote_command(
            "/home/alice/Bobine/install.sh",
            &InstallOptions::assistant_default(),
            &Elevation::Sudo,
        );
        assert_eq!(
            cmd,
            "sudo /home/alice/Bobine/install.sh -y --progress=json --channel=stable"
        );
    }

    #[test]
    fn su_root_injects_as_user_and_wraps() {
        let cmd = build_remote_command(
            "/home/alice/Bobine/install.sh",
            &InstallOptions::assistant_default(),
            &Elevation::SuRoot {
                as_user: "alice".to_string(),
            },
        );
        assert_eq!(
            cmd,
            "su - -c '/home/alice/Bobine/install.sh --as-user alice -y --progress=json --channel=stable'"
        );
    }

    #[test]
    fn paths_with_spaces_are_quoted() {
        let cmd = build_remote_command(
            "/home/fa nta/Bobine/install.sh",
            &InstallOptions::assistant_default(),
            &Elevation::Sudo,
        );
        assert_eq!(
            cmd,
            "sudo '/home/fa nta/Bobine/install.sh' -y --progress=json --channel=stable"
        );
    }

    #[test]
    fn single_quote_in_token_is_escaped() {
        assert_eq!(shell_single_quote("a'b"), r#"'a'\''b'"#);
    }

    #[test]
    fn su_root_quoting_survives_space_in_login_home() {
        let cmd = build_remote_command(
            "/opt/bobine/install.sh",
            &InstallOptions {
                no_kiosk: true,
                progress_json: true,
                ..Default::default()
            },
            &Elevation::SuRoot {
                as_user: "op er".to_string(),
            },
        );
        assert_eq!(
            cmd,
            "su - -c '/opt/bobine/install.sh --as-user '\\''op er'\\'' -y --progress=json --no-kiosk'"
        );
    }

    #[test]
    fn uploaded_script_runs_through_an_explicit_interpreter() {
        let sudo = build_remote_command_via(
            "bash",
            "/tmp/bobine-install-1.sh",
            &InstallOptions::assistant_default(),
            &Elevation::Sudo,
        );
        assert_eq!(
            sudo,
            "sudo bash /tmp/bobine-install-1.sh -y --progress=json --channel=stable"
        );
        let su = build_remote_command_via(
            "bash",
            "/tmp/bobine-install-1.sh",
            &InstallOptions::assistant_default(),
            &Elevation::SuRoot {
                as_user: "alice".to_string(),
            },
        );
        assert_eq!(
            su,
            "su - -c 'bash /tmp/bobine-install-1.sh --as-user alice -y --progress=json --channel=stable'"
        );
    }
}
