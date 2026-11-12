//! Ligne de commande : une sous-commande par commande du catalogue, traduite pour le moteur.
//!
//! Contrat, comme la commande `bobine` : résultat lisible ou `--json` sur stdout, messages sur stderr, codes de sortie
//! 0 succès, 1 échec, 2 usage, 3 cible injoignable, 4 clé d'hôte ou authentification refusée.

/// `println!` qui ne panique pas quand la sortie est fermée (`bobine-deploy scan | head`).
macro_rules! out {
    ($($arg:tt)*) => {{
        let _ = writeln!(std::io::stdout().lock(), $($arg)*);
    }};
}

mod args;
mod commands;
mod input;
mod output;

use std::io::Write;

use bobine_deploy_engine::execute;
use bobine_deploy_proto::{self as proto, Error, ErrorKind};
use clap::Parser;
use serde_json::json;

use args::Cli;
use output::Console;

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_UNREACHABLE: i32 = 3;
pub const EXIT_REFUSED: i32 = 4;

/// Vrai quand les arguments demandent la ligne de commande : une sous-commande du catalogue, ou `--help`/`--version`.
pub fn is_invocation(args: &[String]) -> bool {
    args.iter().skip(1).any(|a| {
        proto::CATALOG.iter().any(|c| c.name == a) || matches!(a.as_str(), "-h" | "--help" | "-V" | "--version")
    })
}

pub fn main(args: Vec<String>) -> i32 {
    attach_console();
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(e) => {
            let code = if e.use_stderr() { EXIT_USAGE } else { EXIT_OK };
            let _ = e.print();
            return code;
        }
    };
    let json = cli.json;
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("bobine-deploy : démarrage impossible : {e}");
            return EXIT_ERROR;
        }
    };
    match runtime.block_on(run(cli)) {
        Ok(()) => EXIT_OK,
        Err(error) => {
            let code = exit_code(error.kind);
            if json {
                out!(
                    "{}",
                    json!({ "event": "error", "data": { "code": code, "kind": error.kind, "message": error.message } })
                );
            }
            eprintln!("bobine-deploy : {error}");
            code
        }
    }
}

pub fn exit_code(kind: ErrorKind) -> i32 {
    match kind {
        ErrorKind::Unreachable => EXIT_UNREACHABLE,
        ErrorKind::Refused => EXIT_REFUSED,
        ErrorKind::Usage => EXIT_USAGE,
        ErrorKind::Failed => EXIT_ERROR,
    }
}

async fn run(cli: Cli) -> Result<(), Error> {
    let console = Console { json: cli.json };
    let command = commands::command(cli.command).await?;
    let result = execute(command, &console).await?;
    if cli.json {
        output::print_json(&result);
    } else {
        output::print_human(&result);
    }
    Ok(())
}

/// Sous Windows, l'exécutable graphique n'a pas de console : la ligne de commande se rattache à celle du terminal qui
/// l'a lancée, sinon rien ne s'afficherait.
pub fn attach_console() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS);
    }
    let _ = std::io::stdout().flush();
}

/// Noms des sous-commandes, pour le test de parité.
#[cfg(test)]
pub fn subcommand_names() -> Vec<String> {
    use clap::CommandFactory;
    args::Cli::command()
        .get_subcommands()
        .map(|c| c.get_name().to_string())
        .collect()
}

#[cfg(test)]
mod tests;
