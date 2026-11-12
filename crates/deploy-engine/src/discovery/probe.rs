//! Sondes TCP d'un hôte : ports usuels, bannière SSH, signature d'un backend Bobine.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bobine_deploy_core::discovery::{os_hint_from_banner, os_hint_from_ports};
use bobine_deploy_proto::Device;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

/// SSH, HTTP(S), partage Windows, Bobine (1903, 8000 avant la 5.1.0), bureau à distance, AirPlay, impression.
const COMMON_PORTS: [u16; 10] = [22, 80, 443, 445, 1903, 3389, 5000, 7000, 8000, 9100];
/// Port presque toujours fermé : un refus rapide prouve qu'un hôte existe même sans service ouvert.
const LIVENESS_PORT: u16 = 9;
/// Sous la limite de descripteurs d'un processus graphique macOS (256) : au-delà, `connect` échoue et le scan ne
/// trouverait rien.
pub const MAX_OPEN_SOCKETS: usize = 96;
pub const MAX_CONCURRENT_HOSTS: usize = 40;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(900);

enum Outcome {
    Open(TcpStream),
    Refused,
    Silent,
}

/// `None` si rien n'indique qu'un hôte existe à cette adresse.
pub async fn probe_host(sockets: Arc<Semaphore>, ip: Ipv4Addr) -> Option<Device> {
    let mut probes = Vec::with_capacity(COMMON_PORTS.len() + 1);
    for port in COMMON_PORTS.into_iter().chain([LIVENESS_PORT]) {
        let sockets = sockets.clone();
        probes.push(tokio::spawn(async move { (port, connect(&sockets, ip, port).await) }));
    }
    let mut open_ports = Vec::new();
    let mut banner = None;
    let mut bobine_open = false;
    let mut alive = false;
    for probe in probes {
        let Ok((port, outcome)) = probe.await else { continue };
        match outcome {
            Outcome::Open(stream) => {
                alive = true;
                if port == LIVENESS_PORT {
                    continue;
                }
                open_ports.push(port);
                if port == 22 {
                    banner = Some(ssh_banner(stream).await);
                } else if port == 1903 || port == 8000 {
                    bobine_open |= is_bobine(stream).await;
                }
            }
            Outcome::Refused => alive = true,
            Outcome::Silent => {}
        }
    }
    if !alive {
        return None;
    }
    open_ports.sort_unstable();
    let os_hint = banner
        .as_deref()
        .and_then(os_hint_from_banner)
        .or_else(|| os_hint_from_ports(&open_ports));
    Some(device(
        ip,
        None,
        banner.is_some(),
        bobine_open,
        open_ports,
        os_hint,
        "ports",
    ))
}

pub fn device(
    ip: Ipv4Addr,
    hostname: Option<String>,
    ssh_open: bool,
    bobine_open: bool,
    open_ports: Vec<u16>,
    os_hint: Option<String>,
    seen_via: &str,
) -> Device {
    let looks_like_target = os_hint.as_deref().is_some_and(|hint| {
        let hint = hint.to_lowercase();
        hint.contains("debian") || hint.contains("linux") || hint.contains("bobine")
    });
    Device {
        ip: ip.to_string(),
        hostname,
        ssh_open,
        bobine_open,
        open_ports,
        os_hint,
        is_wyse_or_bobine: bobine_open || looks_like_target,
        mac: None,
        seen_via: seen_via.to_string(),
    }
}

async fn connect(sockets: &Semaphore, ip: Ipv4Addr, port: u16) -> Outcome {
    let Ok(_permit) = sockets.acquire().await else {
        return Outcome::Silent;
    };
    match timeout(
        CONNECT_TIMEOUT,
        TcpStream::connect(SocketAddr::new(IpAddr::V4(ip), port)),
    )
    .await
    {
        Ok(Ok(stream)) => Outcome::Open(stream),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => Outcome::Refused,
        _ => Outcome::Silent,
    }
}

/// Lecture bornée : un appareil du réseau ne doit pas pouvoir envoyer une ligne sans fin.
async fn ssh_banner(stream: TcpStream) -> String {
    let mut reader = BufReader::new(stream.take(4096));
    let mut line = String::new();
    match timeout(Duration::from_millis(500), reader.read_line(&mut line)).await {
        Ok(Ok(_)) if !line.trim().is_empty() => line.trim().to_string(),
        _ => "SSH-Open".to_string(),
    }
}

/// `/api/health` de Bobine liste ses `components` : un simple port ouvert peut être n'importe quel serveur.
async fn is_bobine(mut stream: TcpStream) -> bool {
    let request = b"GET /api/health HTTP/1.0\r\nHost: bobine\r\nConnection: close\r\n\r\n";
    if timeout(Duration::from_millis(600), stream.write_all(request))
        .await
        .is_err()
    {
        return false;
    }
    // En-têtes et corps peuvent arriver en plusieurs segments : lire jusqu'au marqueur, à la fin ou à 4 Kio.
    let read = async {
        let mut response = Vec::with_capacity(1024);
        let mut chunk = [0u8; 1024];
        while response.len() < 4096 {
            match stream.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => response.extend_from_slice(&chunk[..n]),
            }
            if String::from_utf8_lossy(&response).contains("\"components\"") {
                return true;
            }
        }
        false
    };
    timeout(Duration::from_millis(900), read).await.unwrap_or(false)
}
