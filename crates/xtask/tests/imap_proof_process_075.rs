use std::process::Command;
use std::time::Duration;
use xtask::imap_proof_process_075::{execute, FailureKind, ProcessBounds};

#[test]
fn observed_process_runs_bounded_probes_and_preserves_cleanup_on_refusal() {
    use xtask::imap_proof_process_075::execute_observed;
    let mut saw_output = false;
    let result = execute_observed(command("echo"), b"observed", bounds(), |observation| {
        assert!(observation.pid > 0);
        assert!(observation.stderr.is_empty());
        assert!(observation.elapsed < bounds().timeout);
        saw_output |= observation.stdout == b"observed";
        Ok(())
    })
    .unwrap();
    assert!(saw_output);
    assert!(result.direct_child_reaped);
    assert_eq!(result.remaining_live_members, 0);
    let error = execute_observed(command("sleep"), &[], bounds(), |_| {
        Err("probe refused".into())
    })
    .unwrap_err();
    assert_eq!(error.kind, FailureKind::Observation);
    assert_eq!(error.detail, "probe refused");
    // An observer cannot make an expired run successful, including after root exit.
    let mut short = bounds();
    short.timeout = Duration::from_millis(10);
    assert_eq!(
        execute_observed(command("echo"), &[], short, |_| {
            std::thread::sleep(Duration::from_millis(20));
            Ok(())
        })
        .unwrap_err()
        .kind,
        FailureKind::Timeout
    );
}

#[test]
fn observer_refusal_after_a_real_descendant_is_announced_reaps_the_owned_tree() {
    use xtask::imap_proof_process_075::execute_observed;
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("descendant");
    let mut worker = command("hold_pipe");
    worker.arg(&file);
    let error = execute_observed(worker, &[], bounds(), |observation| {
        if observation.stdout.starts_with(b"descendant:") {
            return Err("reject announced descendant".into());
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.kind, FailureKind::Observation);
    let pid = std::fs::read_to_string(file)
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(!live(pid));
}
fn command(mode: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_imap-proof-worker-075"));
    c.arg(mode);
    c
}
#[test]
fn outer_refusal_reaps_a_cooperative_nested_controller_group() {
    use xtask::imap_proof_process_075::execute_observed;
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("nested-pid");
    let mut worker = command("nested-controller");
    worker.arg(&file);
    let mut nested_pid = None;
    let failure = execute_observed(worker, &[], bounds(), |_| {
        if let Ok(pid) = std::fs::read_to_string(&file) {
            nested_pid = Some(pid.parse::<u32>().unwrap());
            return Err("nested ownership canary".into());
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(failure.kind, FailureKind::Observation);
    assert!(!live(nested_pid.expect("nested child started")));
}
#[test]
fn terminal_nested_fixture_can_also_complete_and_reap_normally() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("completed-pid");
    let mut worker = command("nested-controller-complete");
    worker.arg(&file);
    let result = execute(worker, &[], bounds()).unwrap();
    assert!(result.direct_child_reaped);
    assert_eq!(result.remaining_live_members, 0);
    let pid = std::fs::read_to_string(file).unwrap().parse().unwrap();
    assert!(!live(pid));
}
fn bounds() -> ProcessBounds {
    ProcessBounds {
        timeout: Duration::from_secs(3),
        cleanup_timeout: Duration::from_secs(2),
        stdout_bytes: 8192,
        stderr_bytes: 8192,
        ..ProcessBounds::default()
    }
}

#[test]
fn managed_proof_worker_echoes_binary_input_without_io_threads_and_reaps_root() {
    let input = [0, 255, 10, 13, 42];
    let result = execute(command("echo"), &input, bounds()).unwrap();
    assert_eq!(result.stdout, input);
    assert!(result.stderr.is_empty());
    assert!(result.status.success());
    assert!(result.direct_child_reaped);
    assert_eq!(result.remaining_live_members, 0);
    assert_eq!(result.io_threads, 0);
    assert!(!result.scope.is_empty());
}

#[test]
fn managed_proof_commands_remove_injection_environment_and_preserve_ordinary_values() {
    let mut process = command("environment");
    process
        .env(
            "JAVA_TOOL_OPTIONS",
            "-javaagent:/missing/proof-injection.jar",
        )
        .env("JDK_JAVA_OPTIONS", "--invalid-proof-option")
        .env("_JAVA_OPTIONS", "-XX:InvalidProofOption")
        .env("CLASSPATH", "/missing/unrecorded-proof.jar")
        .env("LD_PRELOAD", "/missing/proof-injection.so")
        .env("LD_AUDIT", "/missing/proof-audit.so")
        .env("LD_LIBRARY_PATH", "/missing/unrecorded-proof-libraries")
        .env("IMAP_PROOF_TEST_SENTINEL", "preserved-075");
    let output = execute(process, &[], bounds()).unwrap();
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        vec![
            "JAVA_TOOL_OPTIONS=false",
            "JDK_JAVA_OPTIONS=false",
            "_JAVA_OPTIONS=false",
            "CLASSPATH=false",
            "LD_PRELOAD=false",
            "LD_AUDIT=false",
            "LD_LIBRARY_PATH=false",
            "sentinel=preserved-075",
        ]
    );
    assert!(output.stderr.is_empty());
    assert!(output.direct_child_reaped);
    assert_eq!(output.remaining_live_members, 0);
    assert_eq!(output.io_threads, 0);
}
#[test]
fn managed_process_refuses_input_bounds_spawn_and_nonzero_exit() {
    let mut invalid = bounds();
    invalid.timeout = Duration::ZERO;
    assert_eq!(
        execute(command("echo"), &[], invalid).unwrap_err().kind,
        FailureKind::InvalidBounds
    );
    let mut small = bounds();
    small.input_bytes = 1;
    assert_eq!(
        execute(command("echo"), &[0, 1], small).unwrap_err().kind,
        FailureKind::InputLimit
    );
    assert!(execute(
        Command::new("missing-imap-proof-program-075"),
        &[],
        bounds()
    )
    .is_err());
    let error = execute(command("error"), &[], bounds()).unwrap_err();
    assert_eq!(error.kind, FailureKind::Exit);
    assert!(error.detail.contains("fixture_exit"));
    assert_eq!(
        execute(command("future_mode"), &[], bounds())
            .unwrap_err()
            .kind,
        FailureKind::Exit
    );
}
#[test]
fn stdout_stderr_floods_are_terminated_at_their_own_byte_budgets() {
    for (mode, kind) in [
        ("stdout", FailureKind::StdoutLimit),
        ("stderr", FailureKind::StderrLimit),
    ] {
        assert_eq!(
            execute(command(mode), &[], bounds()).unwrap_err().kind,
            kind
        );
    }
}
#[test]
fn inherited_pipes_are_closed_by_tree_cleanup_after_direct_worker_exit() {
    let directory = tempfile::tempdir().unwrap();
    let pid_file = directory.path().join("descendant.pid");
    let mut child = command("hold_pipe");
    child.arg(&pid_file);
    let result = execute(child, &[], bounds()).unwrap();
    assert!(result.stdout.starts_with(b"descendant:"));
    assert_eq!(result.remaining_live_members, 0);
    assert!(result.direct_child_reaped);
    assert_eq!(result.io_threads, 0);
    let pid = std::fs::read_to_string(pid_file)
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert!(!live(pid));
}
#[test]
fn ignored_input_and_hung_execution_share_the_same_deadline_without_writer_leaks() {
    for mode in ["sleep", "ignore_stdin"] {
        let mut limit = bounds();
        limit.timeout = Duration::from_millis(150);
        assert_eq!(
            execute(command(mode), &vec![0; 1024 * 1024], limit)
                .unwrap_err()
                .kind,
            FailureKind::Timeout
        );
    }
}
#[test]
fn platform_memory_envelope_refuses_a_worker_oversized_allocation() {
    let mut limit = bounds();
    limit.memory_bytes = 128 * 1024 * 1024;
    let error = execute(command("memory"), &[], limit).unwrap_err();
    assert_eq!(error.kind, FailureKind::Exit);
}

#[test]
fn process_count_cap_is_enforced_or_explicitly_refused_without_silent_downgrade() {
    let mut limits = bounds();
    limits.maximum_processes = 1;
    let error = execute(command("hold_pipe"), &[], limits).unwrap_err();
    assert_eq!(
        error.kind,
        if cfg!(windows) {
            FailureKind::Exit
        } else {
            FailureKind::UnsupportedPlatform
        }
    );
}
#[cfg(windows)]
fn live(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0;
        let okay = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        okay != 0 && code == 259
    }
}
#[cfg(target_os = "linux")]
fn live(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| {
            s.rsplit_once(')').map(|(_, tail)| {
                tail.trim_start()
                    .chars()
                    .next()
                    .is_some_and(|c| c != 'Z' && c != 'X')
            })
        })
        .unwrap_or(false)
}
