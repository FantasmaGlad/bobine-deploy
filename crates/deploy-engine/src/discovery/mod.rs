//! Balayage du réseau local : tout ce qui répond est listé (routeur, imprimante, téléphones…), les cibles probables en
//! tête, pour que l'utilisateur reconnaisse sa borne au lieu de croire à un scan incomplet.

mod neighbours;
mod plan;
mod probe;

use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bobine_deploy_proto::{Device, Error, Event, ScanProgress};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::Sink;
use plan::ScanPlan;

/// Au plus environ huit mises à jour par seconde : fluide sans noyer l'interface.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);

pub async fn scan(sink: &dyn Sink, custom_subnet: Option<String>) -> Result<Vec<Device>, Error> {
    let plan = ScanPlan::new(custom_subnet.as_deref())?;
    let candidates = plan.candidates();
    let labels = plan.labels();
    let mut devices = probe_all(&candidates, &labels, sink).await;
    add_neighbours(&mut devices, &candidates, sink);
    add_bobine_local(&mut devices, sink).await;
    resolve_names(&mut devices).await;
    sort(&mut devices);
    sink.event(Event::ScanProgress(ScanProgress {
        done: candidates.len(),
        total: candidates.len(),
        found: devices.len(),
        networks: labels,
    }));
    Ok(devices)
}

async fn probe_all(candidates: &[Ipv4Addr], labels: &[String], sink: &dyn Sink) -> Vec<Device> {
    let sockets = Arc::new(Semaphore::new(probe::MAX_OPEN_SOCKETS));
    let hosts = Arc::new(Semaphore::new(probe::MAX_CONCURRENT_HOSTS));
    let mut tasks = JoinSet::new();
    for ip in candidates.iter().copied() {
        let (sockets, hosts) = (sockets.clone(), hosts.clone());
        tasks.spawn(async move {
            let _slot = hosts.acquire_owned().await.ok()?;
            probe::probe_host(sockets, ip).await
        });
    }
    let mut devices = Vec::new();
    let mut done = 0;
    let mut last = Instant::now();
    while let Some(joined) = tasks.join_next().await {
        done += 1;
        if let Ok(Some(device)) = joined {
            sink.event(Event::ScanDevice(device.clone()));
            devices.push(device);
        }
        if last.elapsed() >= PROGRESS_INTERVAL || done == candidates.len() {
            last = Instant::now();
            sink.event(Event::ScanProgress(ScanProgress {
                done,
                total: candidates.len(),
                found: devices.len(),
                networks: labels.to_vec(),
            }));
        }
    }
    devices
}

/// Adresses MAC, et appareils présents dans la table ARP mais invisibles aux sondes (tout filtré).
fn add_neighbours(devices: &mut Vec<Device>, candidates: &[Ipv4Addr], sink: &dyn Sink) {
    let arp = neighbours::arp_table();
    for device in devices.iter_mut() {
        if let Ok(ip) = device.ip.parse::<Ipv4Addr>() {
            device.mac = arp.get(&ip).cloned();
        }
    }
    let known: HashSet<String> = devices.iter().map(|d| d.ip.clone()).collect();
    let scanned: HashSet<&Ipv4Addr> = candidates.iter().collect();
    for (ip, mac) in &arp {
        if scanned.contains(ip) && !known.contains(&ip.to_string()) {
            let mut device = probe::device(*ip, None, false, false, Vec::new(), None, "arp");
            device.mac = Some(mac.clone());
            sink.event(Event::ScanDevice(device.clone()));
            devices.push(device);
        }
    }
}

async fn add_bobine_local(devices: &mut Vec<Device>, sink: &dyn Sink) {
    let Some(ip) = neighbours::bobine_local().await else {
        return;
    };
    let hint = "Bobine (bobine.local)".to_string();
    match devices.iter_mut().find(|d| d.ip == ip.to_string()) {
        Some(device) => {
            device.is_wyse_or_bobine = true;
            device.os_hint.get_or_insert(hint);
        }
        None => {
            let mut device = probe::device(
                ip,
                Some("bobine.local".into()),
                false,
                false,
                Vec::new(),
                Some(hint),
                "mdns",
            );
            device.is_wyse_or_bobine = true;
            sink.event(Event::ScanDevice(device.clone()));
            devices.push(device);
        }
    }
}

/// Noms d'hôte en parallèle, pour les seuls appareils retenus.
async fn resolve_names(devices: &mut [Device]) {
    let mut tasks = JoinSet::new();
    for (index, device) in devices.iter().enumerate() {
        if let (None, Ok(ip)) = (&device.hostname, device.ip.parse::<Ipv4Addr>()) {
            tasks.spawn(async move { (index, neighbours::hostname(ip).await) });
        }
    }
    while let Some(joined) = tasks.join_next().await {
        if let Ok((index, Some(name))) = joined {
            devices[index].hostname = Some(name);
        }
    }
}

/// Bobine confirmé, puis cible probable, puis SSH ; le reste par adresse (ordre numérique, pas lexical).
fn sort(devices: &mut [Device]) {
    let ip = |d: &Device| d.ip.parse::<Ipv4Addr>().unwrap_or(Ipv4Addr::UNSPECIFIED).octets();
    devices.sort_by(|a, b| {
        b.bobine_open
            .cmp(&a.bobine_open)
            .then_with(|| b.is_wyse_or_bobine.cmp(&a.is_wyse_or_bobine))
            .then_with(|| b.ssh_open.cmp(&a.ssh_open))
            .then_with(|| ip(a).cmp(&ip(b)))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_come_first_then_numeric_order() {
        let device = |ip: &str, bobine: bool, ssh: bool| {
            probe::device(ip.parse().unwrap(), None, ssh, bobine, Vec::new(), None, "ports")
        };
        let mut devices = vec![
            device("10.0.0.10", false, false),
            device("10.0.0.9", false, false),
            device("10.0.0.20", false, true),
            device("10.0.0.30", true, false),
        ];
        sort(&mut devices);
        let order: Vec<&str> = devices.iter().map(|d| d.ip.as_str()).collect();
        assert_eq!(order, ["10.0.0.30", "10.0.0.20", "10.0.0.9", "10.0.0.10"]);
    }
}
