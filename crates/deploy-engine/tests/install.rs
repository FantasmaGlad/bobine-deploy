//! Lecture du terminal d'installation contre un vrai serveur OpenSSH : événements, journal, code de sortie, mot de
//! passe transmis à l'invite et jamais recopié dans le journal.

mod common;

use std::sync::Mutex;

use bobine_deploy_engine::install::stream;
use bobine_deploy_engine::{ssh, Sink};
use bobine_deploy_proto::{Credentials, Event, HostKeyStatus};
use common::TestServer;

struct Collect(Mutex<Vec<Event>>);

impl Sink for Collect {
    fn event(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn terminal_events_logs_exit_code_and_root_password() {
    let Some(server) = TestServer::start() else {
        eprintln!("sshd introuvable : test ignoré");
        return;
    };
    std::env::set_var("BOBINE_SSH_KNOWN_HOSTS", &server.known_hosts);
    let probe = ssh::Session::connect("127.0.0.1", server.port).await.unwrap();
    let fingerprint = probe.host_key_info().unwrap().fingerprint;
    assert_eq!(probe.trust_host_key(&fingerprint).unwrap().status, HostKeyStatus::Known);
    probe.close().await;
    let session = ssh::open(&Credentials {
        host: "127.0.0.1".into(),
        port: server.port,
        username: TestServer::user(),
        password: None,
        key_path: Some(server.user_key.display().to_string()),
        local_keys: false,
    })
    .await
    .unwrap();

    // Faux installateur : invite de mot de passe sans saut de ligne, lecture du mot de passe, puis le contrat.
    let script = server.dir.join("faux-install.sh");
    std::fs::write(
        &script,
        r#"printf 'Mot de passe : '
read -r secret
echo "journal : secret recu ($secret)"
echo '{"bobine":1,"event":"run_begin","total":1}'
echo '{"bobine":1,"event":"step","status":"start","step":1,"total":1,"pct":0,"slug":"essai","title":"Essai"}'
echo '{"bobine":1,"event":"step","status":"ok","step":1,"total":1,"pct":100,"slug":"essai","title":"Essai"}'
echo '{"bobine":1,"event":"run_end","status":"ok","pct":100,"total":1,"healthy":true,"url":"http://bobine.local:1903","port":1903}'
exit 0
"#,
    )
    .unwrap();
    let sink = Collect(Mutex::new(Vec::new()));
    let (state, code) = stream(&session, &format!("sh {}", script.display()), Some("hunter2"), &sink)
        .await
        .unwrap();
    assert_eq!(code, Some(0));
    assert_eq!(state.pct, 100);
    let events = sink.0.into_inner().unwrap();
    let logs: Vec<&String> = events
        .iter()
        .filter_map(|e| match e {
            Event::InstallLog(l) => Some(l),
            _ => None,
        })
        .collect();
    assert!(logs.iter().any(|l| l.contains("secret recu (***)")), "{logs:?}");
    assert!(!logs.iter().any(|l| l.contains("hunter2")));
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::InstallProgress(p) if p.pct == 100)));

    // Échec du script : le code de sortie remonte.
    let sink = Collect(Mutex::new(Vec::new()));
    let (_, code) = stream(&session, "echo 'panne'; exit 7", None, &sink).await.unwrap();
    assert_eq!(code, Some(7));
    session.close().await;
}
