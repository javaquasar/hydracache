#[test]
fn ordinary_owned_probe_cli_refuses_unknown_path_pid_and_command_overrides() {
    for args in [
        vec![],
        vec!["--exec"],
        vec!["--pid", "1"],
        vec!["native", "x32"],
    ] {
        let output =
            std::process::Command::new(env!("CARGO_BIN_EXE_diagnostic_signer_owned_probe"))
                .args(args)
                .output()
                .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"Fixed owned probe refused.\n");
    }
}
