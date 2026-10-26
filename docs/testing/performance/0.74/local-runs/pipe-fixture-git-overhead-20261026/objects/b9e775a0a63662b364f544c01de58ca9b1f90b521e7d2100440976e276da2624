//! Separate opt-in accounting diagnostic; ordinary source readers stay unchanged.
#[cfg(windows)]
#[path = "../../../tools/xtask-package-attribution-074/pipe_capture.rs"]
mod pipe_capture;

const SPEC: &str = "3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs";
const BODY: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/transport-package-20261025/objects/5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805");
const CALLS: usize = 4;

fn selected(args: &[String], name: &str) -> bool {
    args == ["--exact", name, "--nocapture", "--test-threads=1"]
}

#[test]
fn only_exact_diagnostic_argv_can_run_git() {
    for name in ["reference_cpu_child", "accounted_cpu_child"] {
        let good: Vec<_> = ["--exact", name, "--nocapture", "--test-threads=1"]
            .map(str::to_owned)
            .into();
        assert!(selected(&good, name));
        for bad in [
            vec![],
            good[..3].to_vec(),
            vec!["--list".into()],
            vec!["--test-threads=4".into()],
            [good.clone(), vec!["extra".into()]].concat(),
        ] {
            assert!(!selected(&bad, name));
        }
    }
}

#[test]
fn independent_body_and_fixed_command_are_not_response_pins() {
    use sha2::{Digest, Sha256};
    assert_eq!(CALLS, 4);
    assert_eq!(BODY.len(), 23589);
    assert_eq!(
        Sha256::digest(BODY)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>(),
        "5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805"
    );
    assert!(SPEC.starts_with("3e9bcc06ebce3439495200162cb0c1900b8eda53:"));
}

#[test]
fn reference_cpu_child() {
    child("reference_cpu_child", false);
}
#[test]
fn accounted_cpu_child() {
    child("accounted_cpu_child", true);
}

fn child(name: &str, accounted: bool) {
    if !selected(&std::env::args().skip(1).collect::<Vec<_>>(), name) {
        return;
    }
    #[cfg(not(windows))]
    {
        let _ = accounted;
        panic!("Windows accounting diagnostic only");
    }
    #[cfg(windows)]
    {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for call in 0..CALLS {
            let mut command = std::process::Command::new("git");
            command.args(["show", SPEC]).current_dir(&root);
            let (output, phases) = if accounted {
                let (output, phases) = pipe_capture::profiled(&mut command).unwrap();
                (output, Some(phases))
            } else {
                (pipe_capture::control(&mut command).unwrap(), None)
            };
            assert_eq!(output.status.code(), Some(0));
            assert!(output.stderr.is_empty());
            assert_eq!(output.stdout, BODY);
            println!(
                "\nHC074_GIT_CPU {}",
                serde_json::json!({
                "call":call,"accounted":accounted,"spec":SPEC,"stdout_bytes":output.stdout.len(),
                "stderr_bytes":output.stderr.len(),"code":output.status.code(),"phases":phases})
            );
        }
        println!(
            "HC074_GIT_CPU_DONE {}",
            serde_json::json!({"calls":CALLS,"accounted":accounted,
            "git_argv":["git","show",SPEC],"stdin_hold":false,"git_suspended":false,
            "native_bypass":false,"environment_changes":false,"package_pass":false})
        );
    }
}
