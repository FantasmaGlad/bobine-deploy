//! Moteur SSH contre un vrai serveur OpenSSH (voir `common`). Un seul test par fichier modifie
//! `BOBINE_SSH_KNOWN_HOSTS` : les scénarios s'enchaînent dans un même test pour ne pas se marcher dessus.

mod common;

use bobine_deploy_engine::ssh::{self, Session};
use bobine_deploy_proto::{Credentials, ErrorKind, HostKeyStatus};
use common::TestServer;

fn credentials(server: &TestServer) -> Credentials {
    Credentials {
        host: "127.0.0.1".into(),
        port: server.port,
        username: TestServer::user(),
        password: None,
        key_path: Some(server.user_key.display().to_string()),
        local_keys: false,
    }
}

#[tokio::test]
async fn host_key_trust_authentication_exec_and_upload() {
    let Some(server) = TestServer::start() else {
        eprintln!("sshd introuvable : test ignoré");
        return;
    };
    std::env::set_var("BOBINE_SSH_KNOWN_HOSTS", &server.known_hosts);
    let creds = credentials(&server);

    // Machine inconnue : aucun secret n'est envoyé, l'empreinte est celle d'OpenSSH.
    let session = Session::connect("127.0.0.1", server.port).await.unwrap();
    let info = session.host_key_info().unwrap();
    assert_eq!(info.status, HostKeyStatus::Unknown);
    assert_eq!(info.fingerprint, server.host_fingerprint());
    assert_eq!(info.key_type, "ssh-ed25519");
    let refused = ssh::open(&creds)
        .await
        .err()
        .expect("une clé inconnue doit être refusée");
    assert_eq!(refused.kind, ErrorKind::Refused);
    assert!(refused.message.contains(&info.fingerprint));

    // Une autre empreinte n'enregistre rien ; la bonne, si.
    assert_eq!(
        session.trust_host_key("SHA256:faux").unwrap_err().kind,
        ErrorKind::Refused
    );
    assert!(!server.known_hosts.exists());
    assert_eq!(
        session.trust_host_key(&info.fingerprint).unwrap().status,
        HostKeyStatus::Known
    );
    session.close().await;

    // Clé connue : authentification par clé, commande, téléversement.
    let session = ssh::open(&creds).await.unwrap();
    let (output, code) = session.exec("printf 'bonjour'; exit 3").await.unwrap();
    assert_eq!((output.as_str(), code), ("bonjour", Some(3)));
    let remote = server.dir.join("envoi.sh");
    let payload = "#!/bin/sh\necho 'fichier téléversé'\n";
    session
        .upload(&remote.display().to_string(), payload.as_bytes())
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&remote).unwrap(), payload);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&remote).unwrap().permissions().mode() & 0o777, 0o700);
    }
    session.close().await;

    // Mauvaise clé d'utilisateur : refus explicite, sans mot de passe à proposer.
    let mut wrong = creds.clone();
    wrong.key_path = Some(server.dir.join("host_ed25519").display().to_string());
    assert_eq!(ssh::open(&wrong).await.err().unwrap().kind, ErrorKind::Refused);

    // Clé d'hôte changée (réinstallation ou usurpation) : toujours refusée, jamais remplacée.
    let other = server.dir.join("autre");
    std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(&other)
        .status()
        .unwrap();
    let forged = std::fs::read_to_string(other.with_extension("pub")).unwrap();
    let forged_key: Vec<&str> = forged.split_whitespace().take(2).collect();
    std::fs::write(
        &server.known_hosts,
        format!("[127.0.0.1]:{} {} {}\n", server.port, forged_key[0], forged_key[1]),
    )
    .unwrap();
    let session = Session::connect("127.0.0.1", server.port).await.unwrap();
    assert_eq!(session.host_key_info().unwrap().status, HostKeyStatus::Changed);
    let fingerprint = session.host_key_info().unwrap().fingerprint;
    assert_eq!(
        session.trust_host_key(&fingerprint).unwrap_err().kind,
        ErrorKind::Refused
    );
    assert!(ssh::open(&creds).await.err().unwrap().message.contains("a changé"));
}

#[tokio::test]
async fn unreachable_targets_are_reported_as_such() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let error = Session::connect("127.0.0.1", port).await.err().unwrap();
    assert_eq!(error.kind, ErrorKind::Unreachable);
    assert!(error.message.contains("Connexion refusée"));
    assert_eq!(Session::connect("  ", 22).await.err().unwrap().kind, ErrorKind::Usage);
}
