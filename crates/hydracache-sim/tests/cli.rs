use std::process::Command;

#[test]
fn vopr_soak_subcommand_exits_2_on_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_vopr"))
        .args([
            "soak",
            "--master-seed",
            "22530",
            "--budget-ms",
            "0",
            "--steps-per-seed",
            "4",
            "--max-seeds",
            "1",
            "--synthetic-failure-after-seeds",
            "1",
        ])
        .output()
        .expect("vopr binary runs");

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("\"status\":\"failed\""), "{stdout}");
    assert!(stdout.contains("synthetic_soak_failure"), "{stdout}");
    assert!(stdout.contains("vopr --seed"), "{stdout}");
}
