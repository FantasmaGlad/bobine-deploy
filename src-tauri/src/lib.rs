//! Bobine Deploy : interface Tauri et ligne de commande sur les mêmes fonctions.

pub mod cli;
mod commands;
mod error;
mod events;

use commands::connection::test_ssh_connection;
use commands::discovery::scan_network;
use commands::host_key::{ssh_host_key, trust_ssh_host_key};
use commands::install::start_installation;
use commands::ssh_auth::pick_ssh_key_file;
use commands::version::get_app_info;

/// Interface graphique.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_network,
            test_ssh_connection,
            start_installation,
            get_app_info,
            pick_ssh_key_file,
            ssh_host_key,
            trust_ssh_host_key
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors de l'exécution de Bobine Deploy");
}
