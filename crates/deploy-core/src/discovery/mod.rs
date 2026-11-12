//! Arithmétique de découverte réseau, sans entrée ni sortie : réseaux à sonder, tables de voisinage, indice de
//! système d'un appareil trouvé. Les sondes elles-mêmes vivent dans le moteur.

mod neighbours;
mod os_hint;
mod subnet;

pub use neighbours::{parse_arp_a, parse_proc_net_arp};
pub use os_hint::{os_hint_from_banner, os_hint_from_ports};
pub use subnet::{
    host_range, is_scannable_ipv4, is_tunnel_interface, is_virtual_interface, parse_network_spec, prefix_from_netmask,
    MIN_SCAN_PREFIX,
};
