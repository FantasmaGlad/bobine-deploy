use std::net::Ipv4Addr;

pub fn parse_proc_net_arp(content: &str) -> Vec<(Ipv4Addr, String)> {
    content
        .lines()
        .skip(1)
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                return None;
            }
            let ip = cols[0].parse::<Ipv4Addr>().ok()?;
            let flags = u32::from_str_radix(cols[2].trim_start_matches("0x"), 16).ok()?;
            let mac = cols[3];
            if flags & 0x2 == 0 || mac == "00:00:00:00:00:00" {
                return None;
            }
            Some((ip, mac.to_ascii_lowercase()))
        })
        .collect()
}

pub fn parse_arp_a(content: &str) -> Vec<(Ipv4Addr, String)> {
    let mut out = Vec::new();
    for line in content.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("incomplete") || lower.contains("ff:ff:ff:ff:ff:ff") || lower.contains("ff-ff-ff-ff-ff-ff") {
            continue;
        }
        let ip = line
            .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
            .find_map(|tok| tok.parse::<Ipv4Addr>().ok());
        let mac = line
            .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
            .find(|tok| looks_like_mac(tok));
        if let (Some(ip), Some(mac)) = (ip, mac) {
            out.push((ip, mac.replace('-', ":").to_ascii_lowercase()));
        }
    }
    out
}

fn looks_like_mac(tok: &str) -> bool {
    let sep = if tok.contains(':') {
        ':'
    } else if tok.contains('-') {
        '-'
    } else {
        return false;
    };
    let parts: Vec<&str> = tok.split(sep).collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| (1..=2).contains(&p.len()) && p.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_arp_table_keeps_only_complete_entries() {
        let table = "IP address       HW type     Flags       HW address            Mask     Device\n\
                     192.168.1.1      0x1         0x2         AA:BB:CC:00:11:22     *        wlp3s0\n\
                     192.168.1.9      0x1         0x0         00:00:00:00:00:00     *        wlp3s0\n\
                     192.168.1.30     0x1         0x2         dc:a6:32:01:02:03     *        wlp3s0\n";
        assert_eq!(
            parse_proc_net_arp(table),
            vec![
                (Ipv4Addr::new(192, 168, 1, 1), "aa:bb:cc:00:11:22".to_string()),
                (Ipv4Addr::new(192, 168, 1, 30), "dc:a6:32:01:02:03".to_string()),
            ]
        );
    }

    #[test]
    fn arp_a_output_is_parsed_for_macos_and_windows() {
        let macos = "? (192.168.1.5) at a4:83:e7:1:2:3 on en0 ifscope [ethernet]\n\
                     ? (192.168.1.77) at (incomplete) on en0 ifscope [ethernet]\n\
                     ? (192.168.1.255) at ff:ff:ff:ff:ff:ff on en0 ifscope [ethernet]\n";
        assert_eq!(
            parse_arp_a(macos),
            vec![(Ipv4Addr::new(192, 168, 1, 5), "a4:83:e7:1:2:3".to_string())]
        );
        let windows = "Interface : 192.168.1.20 --- 0x7\n  Adresse Internet      Adresse physique      Type\n  192.168.1.1           aa-bb-cc-00-11-22     dynamique\n  192.168.1.255         ff-ff-ff-ff-ff-ff     statique\n";
        assert_eq!(
            parse_arp_a(windows),
            vec![(Ipv4Addr::new(192, 168, 1, 1), "aa:bb:cc:00:11:22".to_string())]
        );
    }
}
