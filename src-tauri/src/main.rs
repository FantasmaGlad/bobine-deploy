// Pas de console supplémentaire sous Windows en version publiée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bobine_deploy_lib::cli;

fn main() {
    // `bobine-deploy scan|host-key|inspect|install|version` : ligne de commande, sans ouvrir de fenêtre.
    let args: Vec<String> = std::env::args().collect();
    if cli::is_invocation(&args) {
        std::process::exit(cli::main(args));
    }
    bobine_deploy_lib::run();
}
