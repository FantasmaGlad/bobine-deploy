use bobine_deploy_proto::{Command, TrustHostKeyRequest};

pub fn request(host: String, port: u16, fingerprint: String) -> Command {
    Command::TrustHostKey(TrustHostKeyRequest {
        host,
        port,
        fingerprint,
    })
}
