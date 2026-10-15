#[test]
fn fixed_root_cli_has_no_caller_selected_inputs() {
    for args in [
        vec![],
        vec!["--exec", "/bin/true"],
        vec!["prepare", "extra"],
        vec!["--fd", "3"],
        vec!["root-exec", "extra"],
        vec!["--root-child", "extra"],
        vec!["root-native", "extra"],
        vec!["root-x32", "extra"],
        vec!["root-i386", "extra"],
        vec!["root-kill-installed", "extra"],
        vec!["root-drop-installed", "extra"],
        vec!["--root-native-child", "extra"],
        vec!["root-loss-initial", "extra"],
        vec!["root-loss-installed", "extra"],
        vec!["root-loss-healthy-installed", "extra"],
        vec!["root-loss-no-exitkill-installed", "extra"],
        vec!["root-loss-unknown"],
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
