//! Fresh-command pipe attribution only; no installed helper/transport mutation.
#[cfg(windows)]
#[path = "../../../tools/xtask-package-attribution-074/pipe_capture.rs"]
mod pipe_capture;

const BODY: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/transport-package-20261025/objects/5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805");
const SPEC: &str = "3e9bcc06ebce3439495200162cb0c1900b8eda53:crates/xtask/tests/diagnostic_artifact_fixture_074.rs";
const BLOCKS: usize = 6;
const WORKERS: usize = 4;

fn order(block: usize) -> [&'static str; 4] {
    if block & 1 == 0 {
        ["control", "profile", "profile", "control"]
    } else {
        ["profile", "control", "control", "profile"]
    }
}

#[test]
fn fixed_protocol_and_independent_body() {
    use sha2::{Digest, Sha256};
    let document: toml::Value = toml::from_str(include_str!(
        "../../../tools/xtask-package-attribution-074/pipe-execution-protocol.toml"
    ))
    .unwrap();
    assert_eq!(document["workers"].as_integer(), Some(WORKERS as i64));
    assert_eq!(document["blocks"].as_integer(), Some(BLOCKS as i64));
    assert_eq!(document["spec"].as_str(), Some(SPEC));
    assert_eq!(document["samples"].as_integer(), Some(96));
    assert_eq!(BODY.len(), 23589);
    assert_eq!(
        Sha256::digest(BODY)
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>(),
        "5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805"
    );
    assert_eq!(order(0), ["control", "profile", "profile", "control"]);
    assert_eq!(order(1), ["profile", "control", "control", "profile"]);
}

#[test]
fn pipe_fixture_child() {
    use std::io::{Read, Write};
    let Ok(mode) = std::env::var("HC074_PIPE_FIXTURE") else {
        return;
    };
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    assert!(input.is_empty(), "fresh output command must see stdin EOF");
    match mode.as_str() {
        "empty" => {}
        "large" => {
            std::io::stdout()
                .write_all(&vec![b'o'; 1024 * 1024])
                .unwrap();
            std::io::stderr()
                .write_all(&vec![b'e'; 1024 * 1024])
                .unwrap();
        }
        "nonzero" => {
            std::io::stdout()
                .write_all(b"original stdout\0\xff\n")
                .unwrap();
            std::io::stderr()
                .write_all(b"original stderr\0\xfe\n")
                .unwrap();
            std::process::exit(7);
        }
        _ => panic!("unknown fixture"),
    }
    // This synthetic child is a byte/exit fixture, not a nested test verdict.
    // Do not append libtest's variable elapsed-time footer to its raw output.
    // Keep exact stream comparisons below; never normalize captured bytes.
    std::io::stdout().flush().unwrap();
    std::io::stderr().flush().unwrap();
    std::process::exit(0);
}

fn fixture_streams_match(mode: &str, stdout: &[u8], stderr: &[u8], code: Option<i32>) -> bool {
    let mut expected_stdout = b"\nrunning 1 test\ntest pipe_fixture_child ... ".to_vec();
    let (out, err, expected_code) = match mode {
        "empty" => (Vec::new(), Vec::new(), 0),
        "nonzero" => (
            b"original stdout\0\xff\n".to_vec(),
            b"original stderr\0\xfe\n".to_vec(),
            7,
        ),
        "large" => (vec![b'o'; 1024 * 1024], vec![b'e'; 1024 * 1024], 0),
        _ => return false,
    };
    expected_stdout.extend_from_slice(&out);
    code == Some(expected_code) && stdout == expected_stdout && stderr == err
}

#[test]
fn independent_fixture_oracle_rejects_payload_status_and_footer_drift() {
    let prefix = b"\nrunning 1 test\ntest pipe_fixture_child ... ";
    for (mode, payload, stderr, code) in [
        ("empty", Vec::new(), Vec::new(), 0),
        (
            "nonzero",
            b"original stdout\0\xff\n".to_vec(),
            b"original stderr\0\xfe\n".to_vec(),
            7,
        ),
        ("large", vec![b'o'; 1024 * 1024], vec![b'e'; 1024 * 1024], 0),
    ] {
        let stdout = [prefix.as_slice(), &payload].concat();
        assert!(fixture_streams_match(mode, &stdout, &stderr, Some(code)));
        assert!(!fixture_streams_match(
            mode,
            &stdout,
            &stderr,
            Some(code + 1)
        ));
        assert!(!fixture_streams_match(mode, &stdout, &stderr, None));
        let mut changed = stdout.clone();
        *changed.last_mut().unwrap() ^= 1;
        assert!(!fixture_streams_match(mode, &changed, &stderr, Some(code)));
        let mut changed = stderr.clone();
        changed.push(0);
        assert!(!fixture_streams_match(mode, &stdout, &changed, Some(code)));
        for footer in [b"finished in 0.00s\n", b"finished in 0.01s\n"] {
            assert!(!fixture_streams_match(
                mode,
                &[stdout.as_slice(), footer].concat(),
                &stderr,
                Some(code)
            ));
        }
        assert!(!fixture_streams_match(
            "unknown",
            &stdout,
            &stderr,
            Some(code)
        ));
    }
}

#[cfg(windows)]
fn fixture(mode: &str) -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "pipe_fixture_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HC074_PIPE_FIXTURE", mode);
    command
}

#[test]
#[cfg(windows)]
fn pipe_semantics_empty_binary_nonzero_and_large_streams() {
    for mode in ["empty", "nonzero", "large"] {
        let original = pipe_capture::control(&mut fixture(mode)).unwrap();
        let (observed, phases) = pipe_capture::profiled(&mut fixture(mode)).unwrap();
        assert_eq!(original.status, observed.status, "{mode}");
        assert_eq!(original.stdout, observed.stdout, "{mode}");
        assert_eq!(original.stderr, observed.stderr, "{mode}");
        for output in [&original, &observed] {
            assert!(
                fixture_streams_match(mode, &output.stdout, &output.stderr, output.status.code()),
                "{mode}: independent exact stdout/stderr/status oracle"
            );
        }
        assert_eq!(
            original.status.code(),
            Some(if mode == "nonzero" { 7 } else { 0 })
        );
        assert_eq!(
            phases.child_start.created_100ns,
            phases.child_end.created_100ns
        );
        assert_eq!(phases.child_start.pid, phases.child_end.pid);
        assert_ne!(phases.child_end.exited_100ns, 0);
        if mode == "large" {
            assert!(observed.stdout.iter().filter(|&&v| v == b'o').count() >= 1024 * 1024);
            assert_eq!(observed.stderr, vec![b'e'; 1024 * 1024]);
        }
    }
}

#[test]
#[cfg(windows)]
fn pipe_spawn_failure_is_not_replaced_or_retried() {
    let missing = "HC074-never-present-executable-6d7f094d.exe";
    let a = pipe_capture::control(&mut std::process::Command::new(missing)).unwrap_err();
    let b = pipe_capture::profiled(&mut std::process::Command::new(missing)).unwrap_err();
    assert_eq!(a.kind(), std::io::ErrorKind::NotFound);
    assert_eq!(a.kind(), b.kind());
    assert_eq!(a.raw_os_error(), b.raw_os_error());
}

#[test]
fn direct_output_cost_child() {
    let Ok(mode) = std::env::var("HC074_PIPE_COST") else {
        return;
    };
    assert_eq!(mode, "v1");
    #[cfg(windows)]
    run();
    #[cfg(not(windows))]
    panic!("this process-CPU diagnostic requires Windows retained handles");
}

#[cfg(windows)]
fn run() {
    use serde_json::json;
    use std::os::windows::io::BorrowedHandle;
    use std::{
        process::Command,
        sync::{Arc, Barrier},
        time::Instant,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let barrier = Arc::new(Barrier::new(WORKERS));
    // SAFETY: GetCurrentProcess returns a pseudo-handle valid in this process.
    // Borrow only; never close it or reopen the process by PID.
    let parent = unsafe { BorrowedHandle::borrow_raw(GetCurrentProcess()) };
    let before = pipe_capture::times(&parent, std::process::id()).unwrap();
    let cohort = Instant::now();
    let mut rows = Vec::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..WORKERS).map(|worker| {
            let barrier = Arc::clone(&barrier);
            let root = &root;
            scope.spawn(move || {
                let mut rows = Vec::new();
                for block in 0..BLOCKS {
                    for (slot, variant) in order(block).into_iter().enumerate() {
                        barrier.wait();
                        let start = Instant::now();
                        let mut command = Command::new("git");
                        command.args(["show", SPEC]).current_dir(root);
                        let setup_ns = start.elapsed().as_nanos();
                        let (output, phases) = if variant == "control" {
                            (pipe_capture::control(&mut command).unwrap(), None)
                        } else {
                            let (output, phases) = pipe_capture::profiled(&mut command).unwrap();
                            (output, Some(phases))
                        };
                        let wall_ns = start.elapsed().as_nanos();
                        assert_eq!(output.status.code(), Some(0));
                        assert!(output.stderr.is_empty());
                        assert_eq!(output.stdout, BODY, "independent bytes, not first-response pins");
                        rows.push(json!({"worker":worker,"block":block,"slot":slot,"variant":variant,
                            "setup_ns":setup_ns,"wall_ns":wall_ns,"phases":phases,
                            "stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len(),"code":output.status.code()}));
                    }
                }
                rows
            })
        }).collect();
        for handle in handles {
            rows.extend(handle.join().unwrap());
        }
    });
    let wall_ns = cohort.elapsed().as_nanos();
    let after = pipe_capture::times(&parent, std::process::id()).unwrap();
    assert_eq!(rows.len(), WORKERS * BLOCKS * 4);
    for row in rows {
        println!("\nHC074_PIPE {row}");
    }
    println!(
        "HC074_PIPE_DONE {}",
        json!({"samples":96,"workers":WORKERS,"blocks":BLOCKS,
        "wall_ns":wall_ns,"parent_cpu_ns":after.kernel_ns+after.user_ns-before.kernel_ns-before.user_ns,
        "spec":SPEC,"body_sha256":"5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805",
        "product_candidate":false,"installed_helper_changed":false,"admission":false})
    );
}
