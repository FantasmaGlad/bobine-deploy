use std::path::Path;

use sha2::{Digest, Sha256};

/// Embarque la version et le commit de build (`VERSION`, `COMMIT` à la racine du dépôt, `COMMIT` écrit par la CI), et
/// refuse de compiler si `vendor/install.sh` n'a pas l'empreinte épinglée dans `bobine-release.toml`.
fn main() {
    tauri_build::build();

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let read = |name: &str, default: &str| {
        std::fs::read_to_string(root.join(name))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default.to_string())
    };
    println!("cargo:rustc-env=BOBINE_VERSION={}", read("VERSION", "0.0.0-dev"));
    println!("cargo:rustc-env=BOBINE_COMMIT={}", read("COMMIT", "unknown"));

    let pin = std::fs::read_to_string(root.join("bobine-release.toml")).expect("bobine-release.toml illisible");
    let expected = pin
        .lines()
        .find_map(|line| line.strip_prefix("install_sh_sha256 = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("install_sh_sha256 absent de bobine-release.toml");
    let script = std::fs::read(root.join("vendor/install.sh")).expect("vendor/install.sh illisible");
    let actual = format!("{:x}", Sha256::digest(&script));
    assert_eq!(
        actual, expected,
        "vendor/install.sh ne correspond pas à bobine-release.toml : scripts/sync-install-sh.py"
    );

    for path in [
        "../VERSION",
        "../COMMIT",
        "../bobine-release.toml",
        "../vendor/install.sh",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
}
