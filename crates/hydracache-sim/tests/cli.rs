use std::process::Command;

#[test]
fn cli_vopr_seed_flag_is_deterministic() {
    let first = run_vopr(["--seed", "44", "--steps", "16"]);
    let second = run_vopr(["--seed", "44", "--steps", "16"]);

    assert_eq!(first, second);
    assert!(first.contains("seed=44"));
    assert!(first.contains("steps=16"));
    assert!(first.contains("invariant_violations=0"));
}

fn run_vopr<const N: usize>(args: [&str; N]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_vopr"))
        .args(args)
        .output()
        .expect("vopr runs");
    assert!(
        output.status.success(),
        "vopr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf8")
}
