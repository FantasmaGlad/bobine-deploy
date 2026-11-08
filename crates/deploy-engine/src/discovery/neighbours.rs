//! Ce que les sondes ne voient pas : table ARP du poste, nom `bobine.local`, noms d'hôte.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;

use bobine_deploy_core::discovery::{is_scannable_ipv4, parse_arp_a};
use tokio::time::timeout;

/// Lue après le balayage : le noyau y inscrit aussi les hôtes dont tous les ports sont filtrés.
pub fn arp_table() -> HashMap<Ipv4Addr, String> {
    #[cfg(target_os = "linux")]
    if let Ok(content) = std::fs::read_to_string("/proc/net/arp") {
        return bobine_deploy_core::discovery::parse_proc_net_arp(&content)
            .into_iter()
            .collect();
    }
    let mut command = std::process::Command::new(arp_program());
    command.arg("-a");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    match command.output() {
        Ok(out) if out.status.success() => parse_arp_a(&String::from_utf8_lossy(&out.stdout)).into_iter().collect(),
        _ => HashMap::new(),
    }
}

/// Chemin absolu : un `arp` placé plus tôt dans le PATH ne doit pas s'exécuter.
fn arp_program() -> PathBuf {
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("System32").join("arp.exe")
    }
    #[cfg(not(windows))]
    {
        ["/usr/sbin/arp", "/sbin/arp", "/usr/bin/arp"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file())
            .unwrap_or_else(|| "/usr/sbin/arp".into())
    }
}

/// `bobine.local`, publié par l'appliance : trouve la borne même sur un sous-réseau relayé par mDNS.
pub async fn bobine_local() -> Option<Ipv4Addr> {
    let lookup = tokio::task::spawn_blocking(|| {
        ("bobine.local", 1903u16)
            .to_socket_addrs()
            .ok()?
            .find_map(|addr| match addr.ip() {
                IpAddr::V4(ip) if is_scannable_ipv4(ip) => Some(ip),
                _ => None,
            })
    });
    timeout(Duration::from_millis(1500), lookup).await.ok()?.ok()?
}

/// Résolution inverse par le résolveur système (DNS, mDNS, NetBIOS selon la plateforme), bornée dans le temps.
pub async fn hostname(ip: Ipv4Addr) -> Option<String> {
    let addr = IpAddr::V4(ip);
    let lookup = tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&addr).ok());
    match timeout(Duration::from_millis(1200), lookup).await {
        Ok(Ok(Some(name))) if name != ip.to_string() => Some(name),
        _ => None,
    }
}
