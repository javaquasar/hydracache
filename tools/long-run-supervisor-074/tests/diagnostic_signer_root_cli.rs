#[test]
fn fixed_root_cli_has_no_caller_selected_inputs() {
    for args in [
        vec![],
        vec!["--exec", "/bin/true"],
        vec!["prepare", "extra"],
        vec!["--fd", "3"],
    ] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_diagnostic_signer_root_probe"))
            .env_clear()
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        assert_eq!(out.stderr, b"Fixed root preparation refused.\n");
    }
}
