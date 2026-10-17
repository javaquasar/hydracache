//! Exact additive join registrations; all accepted live/runtime guard bodies remain frozen.
const RESTORATIONS: &[(&str, &str, &str)] = &[
    ("crates/xtask/tests/performance_contract_074.rs", "\n#[path = \"support/performance_contract_074_output_join.rs\"]\nmod output_join;\n", ""),
    ("crates/xtask/tests/support/performance_contract_074_original_pipes.rs", "    let current = super::output_join::normalize(path, raw);\n    let raw = current.as_str();\n", ""),
    ("crates/xtask/tests/support/performance_contract_074_original_pipes.rs", "        assert_eq!(\n            super::output_join::normalize(p, &read(p)),\n            old(p),\n            \"out of scope: {p}\"\n        );", "        assert_eq!(read(p), old(p), \"out of scope: {p}\");"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs", "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[path = \"diagnostic_original_collector_sinks.rs\"]\npub mod sinks;\n", ""),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs", "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[path = \"diagnostic_original_collector_sink_storage.rs\"]\nmod sink_storage;\n", ""),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs", "\n#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[path = \"diagnostic_original_collector_sinks_tests.rs\"]\nmod sink_tests;\n", ""),
    ("docs/COMPAT.md", "The opt-in joined original collector output layout v1 creates\n`diagnostic-collector-output-074-<sha256 of original Reserved LF bytes>/collector-attempt-v1/{collector-stdout.bin,collector-stderr.bin}`\nunder the original retained `/var/lib/hydracache-performance` parent. Both\ndirectories are exclusive root:root 0700; empty sinks are exclusive root:root\n0600 regular single-link files. File and containing-directory fsync precede\nruntime acknowledgement. Partial or complete context is never adopted,\noverwritten, repaired or automatically removed. Old three-document intent\nhistory and signed collector projections stay unchanged. Retained paths cannot\nrecover an owner, execution permission, completion or a replay right. No\ninstalled consumer, migration or downgrade is provided. The consuming entry is\nLinux x86_64 only; exact F_GETFL includes its kernel-forced large-file bit.\nNonroot modeled tests use only separately retained temporary ancestry.\nThis layout is not an installed production execution capability.\n\n", ""),
    ("docs/testing/gated-test-registry.toml", "\n# BEGIN ORIGINAL OUTPUT JOIN DIAGNOSTIC-ORIGINAL-OUTPUT-JOIN GATE\n[[gate]]\nid = \"cfg.hydracache-long-run-supervisor-074.diagnostic-original-output-join\"\nkind = \"cfg_test_target\"\nsource = \"tools/long-run-supervisor-074/tests/diagnostic_original_output_join.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"diagnostic_original_output_join\"\ntest = \"\"\ncfg = 'all (target_os = \"linux\" , target_arch = \"x86_64\")'\nenv = \"\"\nreason = \"Original constant-depth Starting/pipes/sinks join; no execution or deadline change\"\ntier = \"fast\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 120\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"rust\"\nstep = \"Release 0.74 Linux supervisor contracts\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\",\"-p\",\"hydracache-long-run-supervisor-074\",\"--test\",\"diagnostic_original_output_join\",\"--locked\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT JOIN DIAGNOSTIC-ORIGINAL-OUTPUT-JOIN GATE\n\n# BEGIN ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-COST-AND-HEALTHY-CONTROL GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-output-join-cost-and-healthy-control\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_output_join_cost_and_healthy_control\"\ncfg = 'all (target_os = \"linux\" , target_arch = \"x86_64\")'\nenv = \"\"\nreason = \"Original constant-depth Starting/pipes/sinks join; no execution or deadline change\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\",\"--release\",\"-p\",\"hydracache-long-run-supervisor-074\",\"--lib\",\"--locked\",\"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::original_output_join_cost_and_healthy_control\",\"--\",\"--ignored\",\"--exact\",\"--nocapture\",\"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-COST-AND-HEALTHY-CONTROL GATE\n\n# BEGIN ORIGINAL OUTPUT JOIN ORIGINAL-COLLECTOR-SINKS-JOINED-FAULT-AND-LIVE-REFUSAL GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-collector-sinks-joined-fault-and-live-refusal\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_collector_sinks_joined_fault_and_live_refusal\"\ncfg = 'all (target_os = \"linux\" , target_arch = \"x86_64\")'\nenv = \"\"\nreason = \"Original constant-depth Starting/pipes/sinks join; no execution or deadline change\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\",\"--release\",\"-p\",\"hydracache-long-run-supervisor-074\",\"--lib\",\"--locked\",\"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::original_collector_sinks_joined_fault_and_live_refusal\",\"--\",\"--ignored\",\"--exact\",\"--nocapture\",\"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT JOIN ORIGINAL-COLLECTOR-SINKS-JOINED-FAULT-AND-LIVE-REFUSAL GATE\n\n# BEGIN ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-ALL-EFFECTS-LIVE-REFUSAL GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-output-join-all-effects-live-refusal\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_output_join_fault_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_output_join_all_effects_live_refusal\"\ncfg = 'all (target_os = \"linux\" , target_arch = \"x86_64\")'\nenv = \"\"\nreason = \"Original constant-depth Starting/pipes/sinks join; no execution or deadline change\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\",\"--release\",\"-p\",\"hydracache-long-run-supervisor-074\",\"--lib\",\"--locked\",\"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::join_fault_tests::original_output_join_all_effects_live_refusal\",\"--\",\"--ignored\",\"--exact\",\"--nocapture\",\"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-ALL-EFFECTS-LIVE-REFUSAL GATE\n\n# BEGIN ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-PROCESS-CRASH-ALL-STAGES-AND-LOST-ACK GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-output-join-process-crash-all-stages-and-lost-ack\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_output_join_process_crash_all_stages_and_lost_ack\"\ncfg = 'all (target_os = \"linux\" , target_arch = \"x86_64\")'\nenv = \"\"\nreason = \"Original constant-depth Starting/pipes/sinks join; no execution or deadline change\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\",\"--release\",\"-p\",\"hydracache-long-run-supervisor-074\",\"--lib\",\"--locked\",\"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::original_output_join_process_crash_all_stages_and_lost_ack\",\"--\",\"--ignored\",\"--exact\",\"--nocapture\",\"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-PROCESS-CRASH-ALL-STAGES-AND-LOST-ACK GATE\n\n# BEGIN ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-EXPIRY-SITE-ATTRIBUTION GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.original-output-join-expiry-site-attribution\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_original_output_join_fault_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"original_output_join_expiry_site_attribution\"\ncfg = 'all (target_os = \"linux\" , target_arch = \"x86_64\")'\nenv = \"\"\nreason = \"Original constant-depth Starting/pipes/sinks join; no execution or deadline change\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\",\"--release\",\"-p\",\"hydracache-long-run-supervisor-074\",\"--lib\",\"--locked\",\"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::join_fault_tests::original_output_join_expiry_site_attribution\",\"--\",\"--ignored\",\"--exact\",\"--nocapture\",\"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END ORIGINAL OUTPUT JOIN ORIGINAL-OUTPUT-JOIN-EXPIRY-SITE-ATTRIBUTION GATE\n", ""),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut clean = raw.to_owned();
    for (p, new, old) in RESTORATIONS {
        if path == *p {
            assert_eq!(
                clean.matches(new).count(),
                1,
                "unapproved output join adapter: {path}"
            );
            clean = clean.replacen(new, old, 1);
        }
    }
    clean
}
#[test]
fn output_join_preserves_original_guards_and_effect_closing_checks() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("a8f4d604:{p}")])
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
    let p = "tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs";
    assert!(std::panic::catch_unwind(|| normalize(
        p,
        &read(p).replace("pub mod sinks;", "pub mod unchecked;")
    ))
    .is_err());
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_preconsent_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs",
        "tools/long-run-supervisor-074/src/server.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(read(p), old(p), "out of scope: {p}");
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs");
    let join = read("tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs");
    let compact_join = join.split_whitespace().collect::<String>();
    for required in [
        "self.pipes.join_revalidate()?",
        "collector_output_parent()?",
        "self.inspect(false)?",
        "self.parent()?.sync_all()?",
        "file.sync_all()?",
        "libc::O_EXCL",
        "libc::O_NOFOLLOW",
        "libc::O_CLOEXEC",
        "libc::openat",
        "KERNEL_LARGEFILE",
        "self.pipes.refuse()",
    ] {
        assert!(source.contains(required), "{required}");
    }
    for required in [
        "self.join_outputs_with(0, 0,",
        "self.original.round(|_| {})",
        ".begin_starting()?",
        ".starting_step(at, &raw)?",
        ".acknowledge_starting()?",
        "checkpoint(OutputJoinStage::Starting(at))?",
        "held.refuse()",
        "held: Box::new(held)",
        "self.inspect(true)",
        "self.starting.joined_round()",
    ] {
        assert!(
            compact_join.contains(&required.split_whitespace().collect::<String>()),
            "{required}"
        );
    }
    for raw in [&source, &join] {
        let runtime = raw
            .lines()
            .filter(|l| !l.trim_start().starts_with("///"))
            .collect::<Vec<_>>()
            .join("\n");
        for denied in [
            "pub fn into_",
            "pub fn start_once",
            "impl Clone",
            "impl AsRawFd",
            "SystemTime",
            "Command::",
            "SigningKey",
            "pub fn recover",
            "set_len(",
            "remove_file(",
            "remove_dir",
            "O_TRUNC",
            "File::open(",
            "pub fn pump",
            "pub fn finish",
        ] {
            assert!(!runtime.contains(denied), "{denied}");
        }
    }
    let owner = source
        .split_once("pub struct OriginalCollectorSinks")
        .unwrap()
        .1
        .split_once("impl fmt::Debug")
        .unwrap()
        .0;
    assert!(
        owner.find("sinks: [Option<Object>; 2]").unwrap()
            < owner.find("pipes: OriginalCollectorPipes").unwrap()
    );
    assert!(
        owner.find("root: Option<Object>").unwrap()
            < owner.find("pipes: OriginalCollectorPipes").unwrap()
    );
    assert!(source.contains("self.effect(stage)?;\n            checkpoint(stage)?;\n            self.inspect(false)?;\n            self.pipes.join_revalidate()?;\n            self.inspect(false)?;"));
}
