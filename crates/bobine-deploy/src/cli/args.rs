use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "bobine-deploy",
    about = "Installer Bobine sur les machines du réseau. Sans argument : interface graphique ; `rpc` : JSON-RPC sur stdio.",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Scan {
        #[arg(long)]
        subnet: Option<String>,
    },
    HostKey {
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
    },
    TrustHostKey {
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
        #[arg(long)]
        fingerprint: String,
    },
    Inspect {
        #[command(flatten)]
        target: Target,
    },
    Install {
        #[command(flatten)]
        target: Target,
        #[arg(long, value_enum, default_value_t = Channel::Stable)]
        channel: Channel,
        #[arg(long, value_enum, default_value_t = Elevation::Auto)]
        elevation: Elevation,
        #[arg(long)]
        no_kiosk: bool,
        #[arg(long)]
        skip_packages: bool,
        #[arg(long, value_name = "CHEMIN")]
        script: Option<String>,
        #[arg(long)]
        simulate: bool,
        #[arg(long)]
        root_password_stdin: bool,
        #[arg(long)]
        yes: bool,
    },
    Version,
}

#[derive(clap::Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub host: String,
    #[arg(long, default_value_t = 22)]
    pub port: u16,
    #[arg(long)]
    pub user: Option<String>,
    #[arg(long, value_name = "FICHIER")]
    pub key: Option<String>,
    #[arg(long)]
    pub local_keys: bool,
    #[arg(long)]
    pub password_stdin: bool,
    #[arg(long, value_name = "EMPREINTE")]
    pub trust_host_key: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Channel {
    Stable,
    Beta,
    Canary,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Elevation {
    Auto,
    Sudo,
    Su,
}
