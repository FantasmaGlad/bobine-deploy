//! Bobine Deploy : sans argument, l'interface graphique ; avec une sous-commande, la ligne de commande ; `rpc`,
//! l'interface d'automatisation JSON-RPC sur stdio. Les trois appellent le même moteur.

// Pas de fenêtre de console à l'ouverture de l'interface sous Windows ; la ligne de commande se rattache à celle du
// terminal qui la lance (`cli::attach_console`).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod cli;
mod parity;
mod rpc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = if args.get(1).map(String::as_str) == Some("rpc") {
        cli::attach_console();
        rpc::main()
    } else if cli::is_invocation(&args) {
        cli::main(args)
    } else {
        cli::main(vec![args[0].clone(), "--help".to_string()])
    };
    std::process::exit(code);
}
