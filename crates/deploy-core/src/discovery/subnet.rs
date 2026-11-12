use std::net::Ipv4Addr;

pub const MIN_SCAN_PREFIX: u8 = 22;

pub fn prefix_from_netmask(mask: Ipv4Addr) -> u8 {
    u32::from(mask).count_ones() as u8
}

pub fn host_range(ip: Ipv4Addr, prefix: u8) -> Vec<Ipv4Addr> {
    let prefix = prefix.clamp(MIN_SCAN_PREFIX, 32);
    if prefix >= 31 {
        return Vec::new();
    }
    let mask: u32 = u32::MAX << (32 - prefix);
    let network = u32::from(ip) & mask;
    let broadcast = network | !mask;
    ((network + 1)..broadcast)
        .map(Ipv4Addr::from)
        .filter(|candidate| *candidate != ip)
        .collect()
}

pub fn parse_network_spec(spec: &str) -> Option<(Ipv4Addr, u8)> {
    let spec = spec.trim();
    let (addr, prefix) = match spec.split_once('/') {
        Some((a, p)) => (a, p.trim().parse::<u8>().ok()?),
        None => (spec, 24),
    };
    if prefix > 32 {
        return None;
    }
    Some((addr.trim().parse::<Ipv4Addr>().ok()?, prefix))
}

pub fn is_scannable_ipv4(ip: Ipv4Addr) -> bool {
    !(ip.is_loopback() || ip.is_link_local() || ip.is_multicast() || ip.is_unspecified() || ip.is_broadcast())
}

pub fn is_virtual_interface(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "docker",
        "br-",
        "veth",
        "virbr",
        "vmnet",
        "vboxnet",
        "vethernet",
        "cni",
        "flannel",
        "podman",
        "lxc",
        "lxd",
    ]
    .iter()
    .any(|prefix| n.starts_with(prefix))
}

pub fn is_tunnel_interface(name: &str, ip: Ipv4Addr) -> bool {
    let n = name.to_ascii_lowercase();
    let by_name = [
        "tun",
        "tap",
        "utun",
        "wg",
        "tailscale",
        "zt",
        "ppp",
        "ipsec",
        "awdl",
        "llw",
    ]
    .iter()
    .any(|prefix| n.starts_with(prefix));
    let o = ip.octets();
    by_name || (o[0] == 100 && (64..=127).contains(&o[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_range_follows_the_real_netmask() {
        let ip = Ipv4Addr::new(192, 168, 1, 77);
        assert_eq!(host_range(ip, 24).len(), 253);
        let wide = host_range(ip, 23);
        assert_eq!(wide.len(), 509);
        assert!(wide.contains(&Ipv4Addr::new(192, 168, 0, 1)));
        assert!(!wide.contains(&ip));
        let capped = host_range(Ipv4Addr::new(10, 0, 5, 9), 16);
        assert_eq!(capped.len(), 1021);
        assert!(capped.contains(&Ipv4Addr::new(10, 0, 4, 1)));
        assert!(capped.contains(&Ipv4Addr::new(10, 0, 7, 254)));
        assert!(!capped.contains(&Ipv4Addr::new(10, 0, 8, 1)));
        assert!(host_range(ip, 31).is_empty());
        assert!(host_range(ip, 32).is_empty());
    }

    #[test]
    fn prefix_is_derived_from_netmask() {
        assert_eq!(prefix_from_netmask(Ipv4Addr::new(255, 255, 255, 0)), 24);
        assert_eq!(prefix_from_netmask(Ipv4Addr::new(255, 255, 254, 0)), 23);
        assert_eq!(prefix_from_netmask(Ipv4Addr::new(255, 255, 0, 0)), 16);
    }

    #[test]
    fn network_spec_accepts_ip_or_cidr() {
        assert_eq!(
            parse_network_spec("192.168.2.10"),
            Some((Ipv4Addr::new(192, 168, 2, 10), 24))
        );
        assert_eq!(
            parse_network_spec(" 10.1.0.0/22 "),
            Some((Ipv4Addr::new(10, 1, 0, 0), 22))
        );
        assert_eq!(parse_network_spec("10.1.0.0/40"), None);
        assert_eq!(parse_network_spec("bobine.local"), None);
    }

    #[test]
    fn interfaces_are_classified() {
        assert!(!is_scannable_ipv4(Ipv4Addr::new(169, 254, 3, 4)));
        assert!(!is_scannable_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_scannable_ipv4(Ipv4Addr::new(192, 168, 1, 2)));
        assert!(is_virtual_interface("docker0"));
        assert!(is_virtual_interface("br-3f2a1b"));
        assert!(!is_virtual_interface("wlp3s0"));
        assert!(is_tunnel_interface("tun0", Ipv4Addr::new(10, 8, 0, 2)));
        assert!(is_tunnel_interface("eth9", Ipv4Addr::new(100, 101, 2, 3)));
        assert!(!is_tunnel_interface("en0", Ipv4Addr::new(192, 168, 1, 2)));
    }
}
