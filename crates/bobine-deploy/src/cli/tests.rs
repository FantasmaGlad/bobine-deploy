use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn subcommands_select_the_command_line() {
    assert!(is_invocation(&args(&["bobine-deploy", "scan"])));
    assert!(is_invocation(&args(&["bobine-deploy", "--json", "version"])));
    assert!(is_invocation(&args(&["bobine-deploy", "--help"])));
    assert!(!is_invocation(&args(&["bobine-deploy"])));
    assert!(!is_invocation(&args(&["bobine-deploy", "-psn_0_12345"])));
}

#[test]
fn usage_errors_and_exit_codes() {
    assert_eq!(main(args(&["bobine-deploy", "install"])), EXIT_USAGE);
    assert_eq!(
        main(args(&[
            "bobine-deploy",
            "install",
            "--host",
            "203.0.113.10",
            "--channel",
            "nightly"
        ])),
        EXIT_USAGE
    );
    assert_eq!(
        main(args(&[
            "bobine-deploy",
            "install",
            "--host",
            "203.0.113.10",
            "--user",
            "admin"
        ])),
        EXIT_USAGE
    );
    assert_eq!(main(args(&["bobine-deploy", "version"])), EXIT_OK);
}

#[test]
fn simulated_installation_runs_without_a_target() {
    let code = main(args(&[
        "bobine-deploy",
        "--json",
        "install",
        "--host",
        "203.0.113.10",
        "--user",
        "admin",
        "--simulate",
    ]));
    assert_eq!(code, EXIT_OK);
}

#[test]
fn unreachable_target_has_its_own_exit_code() {
    let code = main(args(&[
        "bobine-deploy",
        "inspect",
        "--host",
        "127.0.0.1",
        "--port",
        "9",
        "--user",
        "admin",
    ]));
    assert_eq!(code, EXIT_UNREACHABLE);
}
