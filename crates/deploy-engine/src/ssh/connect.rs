//! Connexion TCP et négociation SSH, sans authentification.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bobine_deploy_proto::Error;
use russh::client;
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::{Recorder, Session};

const TCP_TIMEOUT: Duration = Duration::from_secs(8);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
/// Une installation reste parfois silencieuse plusieurs minutes : sans maintien, une box purge la connexion.
const KEEPALIVE: Duration = Duration::from_secs(15);

impl Session {
    /// Connexion TCP (IPv4 d'abord : le scan n'énumère que de l'IPv4) et négociation SSH, sans authentification.
    pub async fn connect(host: &str, port: u16) -> Result<Self, Error> {
        let host = host.trim();
        if host.is_empty() {
            return Err(Error::usage(
                "Adresse de la borne manquante : sélectionnez-la dans le scan ou saisissez son IP.",
            ));
        }
        let mut addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| {
                Error::unreachable(format!(
                    "Nom ou adresse « {host} » introuvable ({e}). Essayez l'adresse IP de la borne (visible dans \
                     l'interface du routeur)."
                ))
            })?
            .collect();
        addrs.sort_by_key(SocketAddr::is_ipv6);
        if addrs.is_empty() {
            return Err(Error::unreachable(format!("Aucune adresse trouvée pour « {host} ».")));
        }

        let mut last_error = String::new();
        let mut stream = None;
        for addr in &addrs {
            match timeout(TCP_TIMEOUT, TcpStream::connect(addr)).await {
                Ok(Ok(tcp)) => {
                    stream = Some(tcp);
                    break;
                }
                Ok(Err(e)) => last_error = describe_connect_error(host, port, &e),
                Err(_) => last_error = describe_timeout(host, port),
            }
        }
        let stream = stream.ok_or_else(|| Error::unreachable(last_error))?;
        stream.set_nodelay(true).ok();

        let config = Arc::new(client::Config {
            keepalive_interval: Some(KEEPALIVE),
            keepalive_max: 8,
            nodelay: true,
            ..Default::default()
        });
        let seen = Arc::new(Mutex::new(None));
        let recorder = Recorder { key: seen.clone() };
        let handle = timeout(HANDSHAKE_TIMEOUT, client::connect_stream(config, stream, recorder))
            .await
            .map_err(|_| Error::unreachable(format!("{host}:{port} ne termine pas la négociation SSH.")))?
            .map_err(|e| {
                Error::unreachable(format!(
                    "Échec de la négociation SSH avec {host}:{port} ({e}) : ce port héberge-t-il bien un serveur SSH ?"
                ))
            })?;
        let host_key = seen
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .ok_or_else(|| Error::failed(format!("{host}:{port} n'a présenté aucune clé d'hôte SSH.")))?;
        Ok(Self {
            handle,
            host: host.to_string(),
            port,
            host_key,
        })
    }
}

fn describe_connect_error(host: &str, port: u16, e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::ConnectionRefused => format!(
            "Connexion refusée par {host}:{port} : la machine est joignable mais SSH n'y est pas actif. Sur la borne : \
             « sudo apt install openssh-server » puis « sudo systemctl enable --now ssh »."
        ),
        std::io::ErrorKind::TimedOut => describe_timeout(host, port),
        _ => format!("Impossible de joindre {host}:{port} : {e}"),
    }
}

fn describe_timeout(host: &str, port: u16) -> String {
    format!(
        "Délai dépassé vers {host}:{port} : machine éteinte, adresse erronée, autre réseau, ou isolation des clients \
         activée sur le Wi-Fi (réglage « isolation AP » ou réseau invité du routeur)."
    )
}
