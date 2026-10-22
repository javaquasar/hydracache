//! Exact narrow restoration; all old public guards remain the independent reference.
const RESTORATIONS: &[(&str, &str, &str)] = &[
    ("crates/xtask/tests/performance_contract_074.rs", "#[path = \"support/performance_contract_074_starting_record.rs\"]\nmod starting_record;\n\n#[path = \"support/performance_contract_074_preconsent_frame.rs\"]\nmod preconsent_frame;\n\n#[test]\nfn ordinary_procless_root_restriction_composition_is_not_credential_admission() {\n    let c = contract(\"diagnostic-signer-owned-root-isolation-contract.toml\");\n", "#[path = \"support/performance_contract_074_starting_record.rs\"]\nmod starting_record;\n\n#[test]\nfn ordinary_procless_root_restriction_composition_is_not_credential_admission() {\n    let c = contract(\"diagnostic-signer-owned-root-isolation-contract.toml\");\n"),
    ("crates/xtask/tests/support/performance_contract_074_starting_record.rs", "    raw.replacen(new, old, 1)\n}\npub(super) fn normalize(path: &str, raw: &str) -> String {\n    let current = super::preconsent_frame::normalize(path, raw);\n    let raw = current.as_str();\n    match path {\n        \"tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs\" => {\n            once(raw, REGISTRATION, \"\", path)\n", "    raw.replacen(new, old, 1)\n}\npub(super) fn normalize(path: &str, raw: &str) -> String {\n    match path {\n        \"tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs\" => {\n            once(raw, REGISTRATION, \"\", path)\n"),
    ("docs/testing/gated-test-registry.toml", "\n[gate.command.env]\n# END COLLECTOR STARTING RECORD GATE\n\n# BEGIN PRECONSENT API GATE\n[[gate]]\nid = \"cfg.hydracache-long-run-supervisor-074.diagnostic-preconsent-frame\"\nkind = \"cfg_test_target\"\nsource = \"tools/long-run-supervisor-074/tests/diagnostic_preconsent_frame.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"diagnostic_preconsent_frame\"\ntest = \"\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original constant-depth consent preparation, no publication, recovery or execution\"\ntier = \"fast\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 120\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"rust\"\nstep = \"Release 0.74 Linux supervisor contracts\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--test\", \"diagnostic_preconsent_frame\", \"--locked\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END PRECONSENT API GATE\n\n# BEGIN PRECONSENT LIVE GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.preconsent-frame-live-mutations-reference-and-consent-refusal\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_preconsent_frame_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"preconsent_frame_live_mutations_reference_and_consent_refusal\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original constant-depth consent preparation, no publication, recovery or execution\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1200\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"--release\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--lib\", \"--locked\", \"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::preconsent_tests::preconsent_frame_live_mutations_reference_and_consent_refusal\", \"--\", \"--ignored\", \"--exact\", \"--nocapture\", \"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END PRECONSENT LIVE GATE\n", "\n[gate.command.env]\n# END COLLECTOR STARTING RECORD GATE\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs", "#[cfg(test)]\n#[path = \"diagnostic_original_start_consent_binding_tests.rs\"]\nmod tests;\n\n#[path = \"diagnostic_preconsent_frame.rs\"]\npub mod preconsent;\n", "#[cfg(test)]\n#[path = \"diagnostic_original_start_consent_binding_tests.rs\"]\nmod tests;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_start_consent_tests.rs", "#[path = \"diagnostic_validation_frame_tests.rs\"]\nmod validation_frame_tests;\n\n#[path = \"diagnostic_preconsent_frame_tests.rs\"]\nmod preconsent_tests;\n\nfn consent(\n    body: crate::diagnostic_start_consent::StartConsentBody,\n    key: &ed25519_dalek::SigningKey,\n", "#[path = \"diagnostic_validation_frame_tests.rs\"]\nmod validation_frame_tests;\n\nfn consent(\n    body: crate::diagnostic_start_consent::StartConsentBody,\n    key: &ed25519_dalek::SigningKey,\n"),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::starting_publication::normalize(path, raw);
    let raw = current.as_str();
    let mut clean = raw.to_owned();
    for (p, new, old) in RESTORATIONS {
        if path == *p {
            assert_eq!(
                clean.matches(new).count(),
                1,
                "unapproved preconsent adapter: {path}"
            );
            clean = clean.replacen(new, old, 1);
        }
    }
    clean
}
#[test]
fn preconsent_frame_preserves_frozen_body_all_lower_guards_and_original_windows() {
    let read = |p: &str| {
        crate::runtime_io::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("ce4a0f54:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for (p, _, _) in RESTORATIONS {
        assert_eq!(normalize(p, &read(p)), old(p), "original body changed: {p}");
        assert_ne!(
            normalize(p, &format!("{}\n// unapproved\n", read(p))),
            old(p)
        );
    }
    let p = "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs";
    assert!(std::panic::catch_unwind(|| normalize(
        p,
        &read(p).replace("pub mod preconsent;", "pub mod unchecked;")
    ))
    .is_err());
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/COMPAT.md",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/src/diagnostic_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_consent_budget_tests.rs",
        "tools/long-run-supervisor-074/src/host_execution.rs",
        "tools/long-run-supervisor-074/src/server.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(
            super::starting_publication::normalize(p, &read(p)),
            old(p),
            "out of scope: {p}"
        );
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_preconsent_frame.rs");
    let reference = old("tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs");
    let slice = |text: &str, start: &str, end: &str| {
        let i = text.find(start).unwrap();
        let tail = &text[i..];
        tail[..tail.find(end).unwrap()].to_owned()
    };
    let compact = |s: &str| s.split_whitespace().collect::<String>();
    let body_start = "let r = &mut self.original.original.reservation;";
    let original = slice(&reference, body_start, "        self.revalidate()");
    let copied = slice(
        &source,
        "let r = &mut self.prepared.original.original.reservation;",
        "        Ok(())",
    )
    .replace("self.prepared.original", "self.original")
    .replace("self.prepared.body", "self.body")
    .replace("self.prepared.point", "self.point");
    assert_eq!(
        compact(&copied).replace(
            "sha256_hex(self.original.original.packet.bytes(),)",
            "sha256_hex(self.original.original.packet.bytes())",
        ),
        compact(&original),
        "frozen projection changed"
    );
    let frame = old("tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs");
    let prepared = slice(&source, "    fn prepared(", "    fn causal(").replace(
        "let prepared = &mut self.prepared;",
        "let prepared = &mut self.original.prepared;",
    );
    assert_eq!(
        compact(&prepared),
        compact(&slice(&frame, "    fn prepared(", "    fn causal("))
    );
    let causal = slice(&source, "    fn causal(", "    fn final_clocks(").replace(
        "let causal = &mut self.prepared.original;",
        "let causal = &mut self.original.prepared.original;",
    );
    assert_eq!(
        compact(&causal),
        compact(&slice(&frame, "    fn causal(", "    fn final_clocks("))
    );
    assert_eq!(
        compact(&slice(&source, "fn valid_age(", "/// Original preparation")),
        compact(&slice(
            &frame,
            "fn valid_age(",
            "impl OriginalFramedConsent"
        ))
    );
    for marker in [
        "self.round(before).and_then(|_| self.round(|_| {}))",
        ".into_validation_frame()",
        ".progress()",
        "self.causal()?",
        "self.prepared()?",
        "self.final_clocks()?",
        "original_reservation_time(Some(&r.last))",
        "r.last = now",
        "CONTROLLER_LOSS_SECONDS",
        "TOTAL_SECONDS",
        "self.prepared.refuse()",
    ] {
        assert!(source.contains(marker), "{marker}");
    }
    assert_eq!(
        source.matches("self.prepared.point = Some(point)").count(),
        1
    );
    let runtime = source
        .lines()
        .take_while(|line| !line.starts_with("#[cfg(test)]"))
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for denied in [
        "std::fs",
        "std::process",
        "SystemTime",
        "SigningKey",
        "pub fn recover",
        "pub fn start_once",
        "pub fn publish",
        "pub fn into_",
        "validated: bool",
        "impl Clone",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
}
