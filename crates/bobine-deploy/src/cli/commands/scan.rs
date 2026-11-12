use bobine_deploy_proto::{Command, ScanRequest};

pub fn request(subnet: Option<String>) -> Command {
    Command::Scan(ScanRequest { subnet })
}
