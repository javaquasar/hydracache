//! Narrow opt-in registrations; accepted publisher and live guards stay exact.
const RESTORATIONS: &[(&str, &str, &str)] = &[
    ("docs/testing/gated-test-registry.toml", "\n# BEGIN ORIGINAL OUTPUT COST GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-output-chain-validation-cost\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_output_chain_validation_cost_attribution\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Single-flight original Starting/pipe validation cost; no execution or deadline change\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"--release\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--lib\", \"--locked\", \"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::pipe_tests::original_output_chain_validation_cost_attribution\", \"--\", \"--ignored\", \"--exact\", \"--nocapture\", \"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT COST GATE\n", ""),
    ("crates/xtask/tests/performance_contract_074.rs", "#[path = \"support/performance_contract_074_starting_publication.rs\"]\nmod starting_publication;\n\n#[path = \"support/performance_contract_074_original_pipes.rs\"]\nmod original_pipes;", "#[path = \"support/performance_contract_074_starting_publication.rs\"]\nmod starting_publication;"),
    ("crates/xtask/tests/support/performance_contract_074_starting_publication.rs", "pub(super) fn normalize(path: &str, raw: &str) -> String {\n    let current = super::original_pipes::normalize(path, raw);\n    let raw = current.as_str();\n    let mut clean = raw.to_owned();", "pub(super) fn normalize(path: &str, raw: &str) -> String {\n    let mut clean = raw.to_owned();"),
    ("crates/xtask/tests/support/performance_contract_074_starting_publication.rs", "            super::original_pipes::normalize(p, &read(p)),\n            expected,\n            \"archived publisher, cuts or workload changed: {p}\"", "            read(p),\n            expected,\n            \"archived publisher, cuts or workload changed: {p}\""),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs", "\n#[path = \"diagnostic_original_collector_pipes.rs\"]\npub mod pipes;\n", ""),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs", "mod crash_tests;\n\n#[path = \"diagnostic_original_collector_pipes_tests.rs\"]\nmod pipe_tests;", "mod crash_tests;"),
    ("docs/testing/gated-test-registry.toml", "\n# BEGIN ORIGINAL PIPE API GATE\n[[gate]]\nid = \"cfg.hydracache-long-run-supervisor-074.diagnostic-original-collector-pipes\"\nkind = \"cfg_test_target\"\nsource = \"tools/long-run-supervisor-074/tests/diagnostic_original_collector_pipes.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"diagnostic_original_collector_pipes\"\ntest = \"\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original anonymous pipes under acknowledged Starting; no FD export, output or execution\"\ntier = \"fast\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 120\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"rust\"\nstep = \"Release 0.74 Linux supervisor contracts\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--test\", \"diagnostic_original_collector_pipes\", \"--locked\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL PIPE API GATE\n\n# BEGIN ORIGINAL PIPE LIVE GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-collector-pipes-joined-fault-and-live-refusal\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_collector_pipes_joined_fault_and_live_refusal\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original anonymous pipes under acknowledged Starting; no FD export, output or execution\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"--release\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--lib\", \"--locked\", \"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::pipe_tests::original_collector_pipes_joined_fault_and_live_refusal\", \"--\", \"--ignored\", \"--exact\", \"--nocapture\", \"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL PIPE LIVE GATE\n", ""),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::output_join::normalize(path, raw);
    let raw = current.as_str();
    let mut clean = raw.to_owned();
    for (p, new, old) in RESTORATIONS {
        if path == *p {
            assert_eq!(
                clean.matches(new).count(),
                1,
                "unapproved original pipe adapter: {path}"
            );
            clean = clean.replacen(new, old, 1);
        }
    }
    clean
}
#[test]
fn original_pipes_preserve_publisher_deadlines_and_private_kernel_ownership() {
    let read = |p: &str| {
        crate::runtime_io::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("a72a1b6a:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for (p, _, _) in RESTORATIONS {
        assert_eq!(
            normalize(p, &read(p)),
            old(p),
            "original source changed: {p}"
        );
        assert_ne!(
            normalize(p, &format!("{}\n// unapproved\n", read(p))),
            old(p)
        );
    }
    let p = "tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs";
    assert!(std::panic::catch_unwind(|| normalize(
        p,
        &read(p).replace("pub mod pipes;", "pub mod unchecked;")
    ))
    .is_err());
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/COMPAT.md",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_preconsent_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs",
        "tools/long-run-supervisor-074/src/server.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(
            super::output_join::normalize(p, &read(p)),
            old(p),
            "out of scope: {p}"
        );
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        "libc::pipe2",
        "libc::O_CLOEXEC",
        "libc::O_NONBLOCK",
        "libc::FD_CLOEXEC",
        "file_type().is_fifo()",
        "m.nlink() != 1",
        "Identity::of(&m) != *expected",
        "self.starting.revalidate()?",
        "self.inspect(true)?",
    ] {
        assert!(source.contains(required), "{required}");
    }
    for denied in [
        "pub fn into_",
        "pub fn start_once",
        "impl Clone",
        "impl AsRawFd",
        "SystemTime",
        "Command::",
        "SigningKey",
        "pub fn recover",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
    assert_eq!(source.matches("self.identity = Some(").count(), 1);
    assert!(source.contains("self.starting.original.refuse()"));
    assert!(source.contains("held: Box::new(held)"));
    let owner = source
        .split_once("pub struct OriginalCollectorPipes")
        .unwrap()
        .1;
    let owner = owner.split_once("impl fmt::Debug").unwrap().0;
    assert!(
        owner.find("pipes: [Option<Pair>; 2]").unwrap()
            < owner.find("starting: OriginalCollectorStarting").unwrap(),
        "pipe ownership must drop before the original fence"
    );
    assert!(source.contains("self.inspect(true)?;\n            before_original();"));
}
