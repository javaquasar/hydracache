#[path = "library.rs"]
mod isolation;
use std::io::Write;
use std::time::Duration;

fn child_barrier(pid: u32) {
    // Bypass libtest's print-macro capture: the parent needs this live barrier,
    // including when the deliberately timed-out test never returns.
    std::io::stdout()
        .lock()
        .write_all(format!("OWNED-CHILD {pid}\n").as_bytes())
        .unwrap();
}

#[test]
fn owned_case_exact_names_and_single_success_cannot_be_forged_by_empty_selection() {
    assert_eq!(
        isolation::case_name("crate_name::nested", "leaf"),
        "nested::leaf"
    );
    assert_eq!(isolation::case_name("crate_name", "leaf"), "leaf");
    let good = "\nrunning 1 test\ntest nested::leaf ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
    assert!(isolation::one_success("nested::leaf", good));
    assert!(isolation::one_success(
        "nested::leaf",
        &good.replace("0 filtered", "5 filtered")
    ));
    for changed in [
        String::new(),
        good.replace("1 passed", "0 passed"),
        good.replace("nested::leaf", "other"),
        good.repeat(2),
        good.replace("0 ignored", "1 ignored"),
        good.replace("0 filtered", "not-a-count filtered"),
    ] {
        assert!(!isolation::one_success("nested::leaf", &changed));
    }
    assert!(!isolation::selected_alone(
        "leaf",
        &["--exact".into(), "leaf".into()]
    ));
    assert!(isolation::selected_alone(
        "leaf",
        &[
            "test-image".into(),
            "--exact".into(),
            "leaf".into(),
            "--test-threads=1".into()
        ]
    ));
    assert!(!isolation::selected_alone(
        "leaf",
        &[
            "test-image".into(),
            "--exact".into(),
            "leaf".into(),
            "--test-threads=1".into(),
            "other".into()
        ]
    ));
    assert!(!isolation::selected_alone(
        "leaf",
        &["--exact".into(), "other".into(), "--test-threads=1".into()]
    ));
}
#[test]
fn owned_case_healthy_child() {
    if isolation::isolate(module_path!(), "owned_case_healthy_child") {
        return;
    }
    assert_eq!(2 + 2, 4);
}
#[test]
fn owned_case_nonzero_child() {
    if isolation::is_child_case(module_path!(), "owned_case_nonzero_child") {
        panic!("controlled owned child refusal");
    }
}
#[test]
fn owned_case_orphan_child() {
    if isolation::is_child_case(module_path!(), "owned_case_orphan_child") {
        let child = std::process::Command::new("/bin/sleep")
            .arg("60")
            .spawn()
            .unwrap();
        child_barrier(child.id());
        std::mem::forget(child);
    }
}
#[test]
fn owned_case_timeout_child() {
    if isolation::is_child_case(module_path!(), "owned_case_timeout_child") {
        let child = std::process::Command::new("/bin/sleep")
            .arg("60")
            .spawn()
            .unwrap();
        child_barrier(child.id());
        std::mem::forget(child);
        std::thread::sleep(Duration::from_secs(60));
    }
}
#[test]
fn owned_case_fail_empty_timeout_and_live_descendant_are_never_positive_proof() {
    for (name, expected, bound) in [
        (
            "owned_case_nonzero_child",
            "child-failed",
            Duration::from_secs(5),
        ),
        (
            "__no_such_case",
            "child-selection-or-result",
            Duration::from_secs(5),
        ),
        (
            "owned_case_orphan_child",
            "unexpected-live-descendant",
            Duration::from_secs(5),
        ),
        (
            "owned_case_timeout_child",
            "timeout-cleanup-is-not-success",
            Duration::from_secs(2),
        ),
    ] {
        let outcome =
            isolation::run_case(&isolation::case_name(module_path!(), name), bound).unwrap();
        assert!(!outcome.success, "{outcome:?}");
        assert_eq!(outcome.fault, Some(expected), "{outcome:?}");
        if matches!(name, "owned_case_orphan_child" | "owned_case_timeout_child") {
            let pid = outcome
                .stdout
                .split("OWNED-CHILD ")
                .nth(1)
                .expect("owned descendant barrier")
                .split_whitespace()
                .next()
                .unwrap();
            let status = std::fs::read_to_string(format!("/proc/{pid}/stat"));
            match status {
                Ok(raw) => assert!(matches!(
                    raw.rsplit_once(')').unwrap().1.split_whitespace().next(),
                    Some("Z" | "X")
                )),
                Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound),
            }
        }
        eprintln!("OWNED-CASE-NEGATIVE {name}: {outcome:?}");
    }
}
