//! Arguments de la ligne de commande.

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "bobine-deploy",
    about = "Installer Bobine sur les machines du réseau. Sans argument : interface graphique ; `rpc` : JSON-RPC sur stdio.",
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Cli {
    /// Sortie JSON sur stdout (une ligne JSON par événement pendant une installation).
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Chercher les machines du réseau local (ou d'un réseau donné).
    Scan {
        /// Adresse ou réseau à sonder (192.168.2.10 ou 192.168.2.0/24) ; défaut : réseaux des interfaces actives.
        #[arg(long)]
        subnet: Option<String>,
    },
    /// Empreinte et statut de la clé d'hôte SSH de la cible, sans s'authentifier.
    HostKey {
        /// Adresse IP ou nom de la cible.
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
    },
    /// Enregistrer la clé d'hôte de la cible si son empreinte est celle donnée.
    TrustHostKey {
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 22)]
        port: u16,
        /// Empreinte vérifiée sur la cible (SHA256:…).
        #[arg(long)]
        fingerprint: String,
    },
    /// Se connecter en SSH et auditer la cible (système, architecture, élévation, matériel).
    Inspect {
        #[command(flatten)]
        target: Target,
    },
    /// Installer Bobine sur la cible (exige --yes).
    Install {
        #[command(flatten)]
        target: Target,
        /// Canal installé sur la cible.
        #[arg(long, value_enum, default_value_t = Channel::Stable)]
        channel: Channel,
        /// Élévation : auto (déterminée par l'audit), sudo, ou su (mot de passe root requis).
        #[arg(long, value_enum, default_value_t = Elevation::Auto)]
        elevation: Elevation,
        /// Sans l'écran plein écran (kiosque) sur la cible.
        #[arg(long)]
        no_kiosk: bool,
        /// Sans installer les paquets système (déjà présents).
        #[arg(long)]
        skip_packages: bool,
        /// Script d'installation déjà présent sur la cible, au lieu du script embarqué.
        #[arg(long, value_name = "CHEMIN")]
        script: Option<String>,
        /// Rejoue une installation enregistrée sans toucher la cible (démonstration, essais).
        #[arg(long)]
        simulate: bool,
        /// Lire le mot de passe root sur l'entrée standard (après celui de --password-stdin).
        #[arg(long)]
        root_password_stdin: bool,
        /// Confirmer l'installation (la cible est modifiée).
        #[arg(long)]
        yes: bool,
    },
    /// Version de Bobine Deploy et du contrat.
    Version,
}

#[derive(clap::Args, Debug)]
pub struct Target {
    /// Adresse IP ou nom de la cible (bobine.local, mini-pc…).
    #[arg(long)]
    pub host: String,
    #[arg(long, default_value_t = 22)]
    pub port: u16,
    /// Utilisateur SSH (défaut : variable USER).
    #[arg(long)]
    pub user: Option<String>,
    /// Clé privée SSH.
    #[arg(long, value_name = "FICHIER")]
    pub key: Option<String>,
    /// Présenter aussi l'agent SSH et les clés usuelles de ~/.ssh.
    #[arg(long)]
    pub local_keys: bool,
    /// Lire le mot de passe SSH sur l'entrée standard.
    #[arg(long)]
    pub password_stdin: bool,
    /// Faire d'abord confiance à la clé d'hôte de la cible si son empreinte est celle-ci (SHA256:…).
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
