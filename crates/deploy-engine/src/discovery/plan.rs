//! Réseaux à sonder : ceux des interfaces actives du poste, ou celui saisi par l'utilisateur.

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};

use bobine_deploy_core::discovery::{
    host_range, is_scannable_ipv4, is_tunnel_interface, is_virtual_interface, parse_network_spec, prefix_from_netmask,
};
use bobine_deploy_proto::Error;

pub struct ScanPlan {
    /// IP du poste, préfixe, libellé affiché (`192.168.1.0/24 (wlp3s0)`).
    pub networks: Vec<(Ipv4Addr, u8, String)>,
    pub local_ips: HashSet<Ipv4Addr>,
}

impl ScanPlan {
    pub fn new(custom_subnet: Option<&str>) -> Result<Self, Error> {
        let mut plan = from_interfaces();
        if let Some(spec) = custom_subnet.map(str::trim).filter(|s| !s.is_empty()) {
            let (ip, prefix) = parse_network_spec(spec).ok_or_else(|| {
                Error::usage("Réseau invalide : saisissez une adresse (192.168.2.10) ou un réseau (192.168.2.0/24).")
            })?;
            let label = format!("{}/{} (saisi)", network_of(ip, prefix), prefix.max(22));
            plan.networks = vec![(ip, prefix, label)];
        }
        if plan.networks.is_empty() {
            return Err(Error::failed(
                "Aucune interface réseau active détectée : branchez-vous au même réseau que la borne (Wi-Fi ou \
                 Ethernet), ou saisissez l'adresse de la borne manuellement.",
            ));
        }
        Ok(plan)
    }

    /// Adresses à sonder, sans doublons entre interfaces ni adresses du poste, triées.
    pub fn candidates(&self) -> Vec<Ipv4Addr> {
        let mut seen = HashSet::new();
        let mut hosts: Vec<Ipv4Addr> = self
            .networks
            .iter()
            .flat_map(|(ip, prefix, _)| host_range(*ip, *prefix))
            .filter(|host| !self.local_ips.contains(host) && seen.insert(*host))
            .collect();
        hosts.sort();
        hosts
    }

    pub fn labels(&self) -> Vec<String> {
        self.networks.iter().map(|(_, _, label)| label.clone()).collect()
    }
}

/// Réseaux physiques d'abord, tunnels seulement s'il n'y a rien d'autre ; jamais les interfaces de conteneurs ou de
/// machines virtuelles. Le préfixe vient du vrai masque : sur un /22, la borne peut être hors du /24 du poste.
fn from_interfaces() -> ScanPlan {
    let mut primary = Vec::new();
    let mut tunnels = Vec::new();
    let mut local_ips = HashSet::new();
    for iface in if_addrs::get_if_addrs().unwrap_or_default() {
        let if_addrs::IfAddr::V4(v4) = &iface.addr else {
            continue;
        };
        if iface.is_loopback() || !is_scannable_ipv4(v4.ip) {
            continue;
        }
        local_ips.insert(v4.ip);
        if is_virtual_interface(&iface.name) {
            continue;
        }
        let prefix = prefix_from_netmask(v4.netmask);
        let label = format!("{}/{} ({})", network_of(v4.ip, prefix), prefix.max(22), iface.name);
        if is_tunnel_interface(&iface.name, v4.ip) {
            tunnels.push((v4.ip, prefix, label));
        } else {
            primary.push((v4.ip, prefix, label));
        }
    }
    let mut networks = if primary.is_empty() { tunnels } else { primary };
    if networks.is_empty() {
        if let Some(ip) = default_route_ip() {
            local_ips.insert(ip);
            networks.push((ip, 24, format!("{}/24 (route par défaut)", network_of(ip, 24))));
        }
    }
    ScanPlan { networks, local_ips }
}

fn network_of(ip: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    let mask = u32::MAX << (32 - prefix.clamp(22, 32));
    Ipv4Addr::from(u32::from(ip) & mask)
}

/// IP de la route par défaut, par une socket UDP connectée : aucun paquet n'est émis.
fn default_route_ip() -> Option<Ipv4Addr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("1.1.1.1:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if is_scannable_ipv4(ip) => Some(ip),
        _ => None,
    }
}
