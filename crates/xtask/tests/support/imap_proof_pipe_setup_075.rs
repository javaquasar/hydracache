use super::*;

fn finite_command() -> Command {
    // A system shell has no LLVM writer to interrupt when setup deliberately
    // fails. The managed parent still owns and verifies its physical cleanup.
    #[cfg(windows)]
    let mut command = Command::new("cmd.exe");
    #[cfg(windows)]
    command.args(["/d", "/c", "echo pipe-ready"]);
    #[cfg(target_os = "linux")]
    let mut command = Command::new("/bin/sh");
    #[cfg(target_os = "linux")]
    command.args(["-c", "printf pipe-ready"]);
    command
}
#[test]
fn pipe_setup_failure_reaps_the_real_owned_child_before_returning_containment_error() {
    for kind in [io::ErrorKind::PermissionDenied, io::ErrorKind::Other] {
        let failure = execute_with_pipe_setup(
            finite_command(),
            &[],
            ProcessBounds::default(),
            ExecutionPolicy::Proof,
            |_| panic!("failed pipe setup must not reach observation"),
            |_, _| Err(io::Error::from(kind)),
        )
        .unwrap_err();
        assert_eq!(failure.kind, FailureKind::Containment);
        assert!(failure.direct_child_reaped);
        assert_eq!(failure.remaining_live_members, Some(0));
    }
}
#[test]
fn actual_pipe_setup_preserves_output_and_input_refusal_precedes_setup() {
    let output = execute_with_pipe_setup(
        finite_command(),
        &[],
        ProcessBounds::default(),
        ExecutionPolicy::Proof,
        |_| Ok(()),
        prepare_pipes,
    )
    .unwrap();
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("pipe-ready"));
    assert!(output.direct_child_reaped);
    assert_eq!(output.remaining_live_members, 0);
    let bounds = ProcessBounds {
        input_bytes: 1,
        ..Default::default()
    };
    let failure = execute_with_pipe_setup(
        finite_command(),
        &[1, 2],
        bounds,
        ExecutionPolicy::Proof,
        |_| panic!("oversized input must not start observation"),
        |_, _| panic!("oversized input must not reach setup or spawn"),
    )
    .unwrap_err();
    assert_eq!(failure.kind, FailureKind::InputLimit);
    assert!(!failure.direct_child_reaped);
    assert_eq!(failure.remaining_live_members, None);
}
