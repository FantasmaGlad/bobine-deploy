use bobine_deploy_proto::{Command, HostKeyRequest};

pub fn request(host: String, port: u16) -> Command {
    Command::HostKey(HostKeyRequest { host, port })
}
