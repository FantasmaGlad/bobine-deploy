#![allow(dead_code)]

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct TestServer {
    pub port: u16,
    pub dir: PathBuf,
    pub user_key: PathBuf,
    pub known_hosts: PathBuf,
    child: Child,
}

fn sshd_binary() -> Option<PathBuf> {
    ["/usr/sbin/sshd", "/usr/local/sbin/sshd", "/opt/homebrew/sbin/sshd"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

fn keygen(path: &Path) {
    let status = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(path)
        .stdout(Stdio::null())
        .status()
        .expect("ssh-keygen introuvable");
    assert!(status.success(), "ssh-keygen a échoué");
}

impl TestServer {
    pub fn start() -> Option<Self> {
        let sshd = sshd_binary()?;
        let dir = std::env::temp_dir().join(format!(
            "bobine-deploy-sshd-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let host_key = dir.join("host_ed25519");
        let user_key = dir.join("user_ed25519");
        keygen(&host_key);
        keygen(&user_key);
        std::fs::copy(dir.join("user_ed25519.pub"), dir.join("authorized_keys")).unwrap();
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let config = dir.join("sshd_config");
        std::fs::write(
            &config,
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\n\
                 StrictModes no\nUsePAM no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\n\
                 PubkeyAuthentication yes\nLogLevel ERROR\n",
                host_key.display(),
                dir.join("sshd.pid").display(),
                dir.join("authorized_keys").display()
            ),
        )
        .unwrap();
        let child = Command::new(sshd)
            .args(["-D", "-e", "-f"])
            .arg(&config)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
            if Instant::now() > deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Some(Self {
            port,
            known_hosts: dir.join("known_hosts"),
            dir,
            user_key,
            child,
        })
    }

    pub fn user() -> String {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .expect("utilisateur courant inconnu")
    }

    pub fn host_fingerprint(&self) -> String {
        let out = Command::new("ssh-keygen")
            .args(["-l", "-E", "sha256", "-f"])
            .arg(self.dir.join("host_ed25519.pub"))
            .output()
            .unwrap();
        String::from_utf8(out.stdout)
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .to_string()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
