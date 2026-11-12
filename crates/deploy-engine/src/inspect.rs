use bobine_deploy_core::privilege::{decide, PrivilegeDecision};
use bobine_deploy_proto::{Credentials, Elevation, Error, Inspection};

use crate::ssh::{self, Session};

pub async fn inspect(creds: &Credentials) -> Result<Inspection, Error> {
    let session = ssh::open(creds).await?;
    let result = audit(&session, &creds.host).await;
    session.close().await;
    result
}

async fn text(session: &Session, command: &str) -> Result<String, Error> {
    Ok(session.exec(command).await?.0)
}

async fn audit(session: &Session, host: &str) -> Result<Inspection, Error> {
    let os_release = text(session, "cat /etc/os-release 2>/dev/null || true").await?;
    let lower = os_release.to_lowercase();
    let is_debian = lower.contains("id=debian") || lower.contains("debian");
    let is_debian_13 = os_release.contains("VERSION_ID=\"13\"") || lower.contains("trixie");
    let os_name = os_release
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_else(|| "Système Linux inconnu".to_string());

    let arch = text(session, "uname -m").await?;
    let is_amd64 = arch.contains("x86_64") || arch.contains("amd64");

    let sudo = crate::install::elevation::sudo_probe(session).await?;
    let (sudo_installed, user_is_sudoer) = (sudo.sudo_installed, sudo.user_is_sudoer);
    let decision = decide(&sudo);

    let cpu_model = text(
        session,
        "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs || uname -p",
    )
    .await
    .unwrap_or_default();
    let memory_kb = text(session, "grep MemTotal /proc/meminfo | awk '{print $2}'")
        .await
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(0);
    let disk_kb = text(session, "df -k / | tail -1 | awk '{print $4}'")
        .await
        .unwrap_or_default()
        .parse::<u64>()
        .unwrap_or(0);
    let gpu_info = text(
        session,
        "lspci 2>/dev/null | grep -E 'VGA|3D|Display' | cut -d: -f3 | xargs || true",
    )
    .await
    .unwrap_or_default();

    Ok(Inspection {
        host: host.trim().to_string(),
        os_name,
        is_debian,
        is_debian_13,
        arch,
        is_amd64,
        sudo_installed,
        user_is_sudoer,
        elevation: match decision {
            PrivilegeDecision::UseSudo => Elevation::Sudo,
            PrivilegeDecision::BootstrapSuAsUser => Elevation::Su,
        },
        needs_root_password: decision == PrivilegeDecision::BootstrapSuAsUser,
        cpu_model: if cpu_model.is_empty() {
            "Processeur inconnu".to_string()
        } else {
            cpu_model
        },
        memory_total_mb: memory_kb / 1024,
        disk_free_gb: disk_kb / (1024 * 1024),
        gpu_info: if gpu_info.is_empty() {
            "Graphismes intégrés".to_string()
        } else {
            gpu_info
        },
    })
}
