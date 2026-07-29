use std::fs;
use std::path::{Path, PathBuf};

use hydracache_loadgen::profile::{
    reference_attestation_problems, RunnerAttestationV3, REFERENCE_RUNNER_CLASS,
};
use xtask::perf_qualification::{qualification_context_problems, QualificationContext};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn context() -> QualificationContext {
    QualificationContext {
        github_actions: "true".to_owned(),
        event_name: "workflow_dispatch".to_owned(),
        git_ref: "refs/heads/main".to_owned(),
        repository: "javaquasar/hydracache".to_owned(),
        head_repository: None,
        workflow_ref: "javaquasar/hydracache/.github/workflows/ci.yml@refs/heads/main".to_owned(),
        performance_mode: "qualify".to_owned(),
        candidate_release: "0.67.1".to_owned(),
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        github_sha: SHA.to_owned(),
        git_head: SHA.to_owned(),
        github_run_id: "123456".to_owned(),
        clean_worktree: true,
    }
}

#[test]
fn qualification_context_rejects_untrusted_events_refs_forks_dirty_and_mismatched_sha() {
    assert!(qualification_context_problems(&context()).is_empty());

    let mut pull_request = context();
    pull_request.event_name = "pull_request".to_owned();
    assert!(!qualification_context_problems(&pull_request).is_empty());

    let mut tag = context();
    tag.git_ref = "refs/tags/v0.67.1".to_owned();
    assert!(!qualification_context_problems(&tag).is_empty());

    let mut fork = context();
    fork.head_repository = Some("someone/hydracache".to_owned());
    assert!(!qualification_context_problems(&fork).is_empty());

    let mut dirty = context();
    dirty.clean_worktree = false;
    assert!(!qualification_context_problems(&dirty).is_empty());

    let mut mismatched = context();
    mismatched.github_sha = "f".repeat(40);
    assert!(!qualification_context_problems(&mismatched).is_empty());

    let mut renamed = context();
    renamed.runner_class = "hydracache-perf-v1".to_owned();
    assert!(!qualification_context_problems(&renamed).is_empty());
}

#[test]
fn qualification_workflow_is_manual_bounded_serialized_and_non_promotable() {
    let workflow = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let implementation =
        fs::read_to_string(root().join("crates/xtask/src/perf_qualification.rs")).unwrap();
    let _: serde_yaml::Value = serde_yaml::from_str(&workflow).unwrap();

    for required in [
        "release-0671-performance-qualification:",
        "inputs.performance_0671_mode == 'qualify'",
        "github.event_name == 'workflow_dispatch'",
        "github.ref == 'refs/heads/main'",
        "runs-on: [self-hosted, linux, x64, hydracache-perf-v1]",
        "group: release-067-performance-reference-v1",
        "timeout-minutes: 360",
        "Run 0.67.1 qualification gate",
        "Upload 0.67.1 qualification diagnostics",
    ] {
        assert!(workflow.contains(required), "workflow lost {required}");
    }
    assert!(implementation.contains("bootstrap_eligible: false"));
    assert!(implementation.contains("ship_evidence_eligible: false"));
    assert!(implementation.contains("QUALIFICATION_LOCAL_RELATIVE_PATH"));
    assert!(implementation.contains("QUALIFICATION_CLIENT_RELATIVE_PATH"));
}

#[test]
fn canary_qualification_accepts_a_vm_with_the_custom_label() {
    let trusted_context = qualification_context_problems(&context()).is_empty();
    let virtualized = RunnerAttestationV3 {
        virtualization: "kvm".to_owned(),
        ..RunnerAttestationV3::default()
    };
    let accepted = trusted_context && reference_attestation_problems(&virtualized).is_empty();

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W3") {
        assert!(
            accepted,
            "HC-CANARY-RED:W3 custom runner label bypassed physical-host attestation"
        );
    } else {
        assert!(!accepted);
    }
}
