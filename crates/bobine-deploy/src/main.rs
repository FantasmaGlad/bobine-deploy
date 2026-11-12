#![cfg_attr(windows, windows_subsystem = "windows")]

mod cli;
mod gui;
mod parity;
mod rpc;
mod text;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = if args.get(1).map(String::as_str) == Some("rpc") {
        cli::attach_console();
        rpc::main()
    } else if cli::is_invocation(&args) {
        cli::main(args)
    } else {
        gui::main()
    };
    std::process::exit(code);
}
