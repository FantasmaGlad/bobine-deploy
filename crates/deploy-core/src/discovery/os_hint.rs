//! Indice du système d'un appareil trouvé, d'après sa bannière SSH ou ses ports ouverts.

/// Indice d'OS grossier déduit d'une bannière SSH (repli `None`).
///
/// Ex. `SSH-2.0-OpenSSH_9.2p1 Debian-2` → `Some("Debian")`.
pub fn os_hint_from_banner(banner: &str) -> Option<String> {
    let b = banner.to_ascii_lowercase();
    if b.contains("raspbian") {
        Some("Raspberry Pi OS".to_string())
    } else if b.contains("ubuntu") {
        Some("Ubuntu".to_string())
    } else if b.contains("debian") {
        Some("Debian".to_string())
    } else if b.contains("openssh") {
        Some("Linux/OpenSSH".to_string())
    } else {
        None
    }
}

/// Indice d'OS de repli déduit des ports TCP ouverts, pour les appareils qui
/// répondent sur le réseau mais n'exposent pas SSH (imprimantes, routeurs,
/// postes Windows, télévisions...). Beaucoup moins fiable qu'une bannière SSH
/// (des heuristiques, pas une identification) — n'intervient qu'en dernier
/// recours quand [`os_hint_from_banner`] n'a rien donné (pas de port 22).
/// Ordre de vérification volontairement du plus spécifique au plus générique.
pub fn os_hint_from_ports(ports: &[u16]) -> Option<String> {
    if ports.contains(&3389) {
        Some("Windows (Bureau à distance)".to_string())
    } else if ports.contains(&445) || ports.contains(&139) {
        Some("Windows / Partage Samba".to_string())
    } else if ports.contains(&5000) && ports.contains(&7000) {
        Some("Apple (AirPlay)".to_string())
    } else if ports.contains(&9100) {
        Some("Imprimante réseau".to_string())
    } else if ports.contains(&1903) || ports.contains(&8000) {
        Some("Bobine (sans SSH)".to_string())
    } else if ports.contains(&80) || ports.contains(&443) {
        Some("Appareil avec interface web".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_hint_recognises_common_banners() {
        assert_eq!(
            os_hint_from_banner("SSH-2.0-OpenSSH_9.2p1 Debian-2+deb12u2"),
            Some("Debian".to_string())
        );
        assert_eq!(
            os_hint_from_banner("SSH-2.0-OpenSSH_8.9p1 Ubuntu-3ubuntu0.1"),
            Some("Ubuntu".to_string())
        );
        // OpenSSH générique sans distro identifiable.
        assert_eq!(
            os_hint_from_banner("SSH-2.0-OpenSSH_for_Windows_8.1"),
            Some("Linux/OpenSSH".to_string())
        );
        assert_eq!(os_hint_from_banner("SSH-2.0-dropbear"), None);
    }

    #[test]
    fn os_hint_from_ports_prioritises_most_specific_signal() {
        assert_eq!(
            os_hint_from_ports(&[80, 445, 3389]),
            Some("Windows (Bureau à distance)".to_string())
        );
        assert_eq!(os_hint_from_ports(&[445]), Some("Windows / Partage Samba".to_string()));
        assert_eq!(os_hint_from_ports(&[9100]), Some("Imprimante réseau".to_string()));
        assert_eq!(os_hint_from_ports(&[1903]), Some("Bobine (sans SSH)".to_string()));
        assert_eq!(os_hint_from_ports(&[8000]), Some("Bobine (sans SSH)".to_string()));
        assert_eq!(
            os_hint_from_ports(&[443]),
            Some("Appareil avec interface web".to_string())
        );
        assert_eq!(os_hint_from_ports(&[]), None);
    }
}
