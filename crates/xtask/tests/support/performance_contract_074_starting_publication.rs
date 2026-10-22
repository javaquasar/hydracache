//! Exact opt-in restoration; no original predicate is exempted.
const RESTORATIONS: &[(&str, &str, &str)] = &[
    ("crates/xtask/tests/performance_contract_074.rs", "#[path = \"support/performance_contract_074_preconsent_frame.rs\"]\nmod preconsent_frame;\n\n#[path = \"support/performance_contract_074_starting_publication.rs\"]\nmod starting_publication;\n", "#[path = \"support/performance_contract_074_preconsent_frame.rs\"]\nmod preconsent_frame;\n"),
    ("crates/xtask/tests/support/performance_contract_074_starting_record.rs", "    ] {\n        assert!(runtime.contains(required), \"{required}\");\n    }\n    let path = \"tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs\";\n    let storage = super::starting_publication::normalize(path, &read(path));\n    assert!(storage.contains(\"let mut seen = [false; 2]\"));\n    assert!(!storage.contains(\"diagnostic-collector-starting-074\"));\n}\n", "    ] {\n        assert!(runtime.contains(required), \"{required}\");\n    }\n    let storage =\n        read(\"tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs\");\n    assert!(storage.contains(\"let mut seen = [false; 2]\"));\n    assert!(!storage.contains(\"diagnostic-collector-starting-074\"));\n}\n"),
    ("docs/COMPAT.md", "| Artifact | Current Version | Writer | Reader Compatibility | Failure Mode |\n| --- | --- | --- | --- | --- |\n| Separate 0.74 collector Starting byte record | `diagnostic-collector-starting-074-v1` | opt-in Linux byte encoder only; no publisher or installed consumer | Canonical LF JSON, at most 16384 bytes; eventual fixed name `diagnostic-collector-starting-074.json`. Binds exact controller-attempt/collector-Reserved LF digests, canonical signed-consent LF digest and the identical signed Starting state; reuses strict prior-history and adjacent-transition grammar. | Unknown/future/duplicate/partial/noncanonical/oversized/mixed bindings refuse. Byte consistency does not verify signatures, inspect actual clocks/credentials or recover acknowledgement, replay ownership or start rights. Old two-file storage and legacy readers remain unchanged and reject a third record. No migration, overwrite, repair, publication, output, execution or cleanup authority. |\n| Separate 0.74 original collector Starting publisher | `diagnostic-collector-starting-074-v1` | opt-in consuming Linux original frame only, not an installed entry | Same byte contract and fixed name; private three-document phase retains original descriptors and fence. Exclusive 0600 create, bounded writes, seal 0400, file/directory fsync and live guards precede runtime acknowledgement. | Default two-file and legacy readers still reject retained Starting history. Partial or complete retained bytes cannot recover acknowledgement or a runtime owner; any uncertainty latches refusal and retains descriptors/context. No repair, retry, migration, process/output start, writer revocation, completion or server admission. |\n| Separate 0.74 collector progress challenge | `diagnostic-collector-progress-challenge-074-v1` | opt-in Linux original-owner library only; no installed consumer | Canonical LF JSON, at most 4096 bytes; 32 system-random bytes, original controller-attempt/Reserved/canonical-state hashes, identity, revision 1 and original-kernel issuance BOOTTIME. SHA256 of `hydracache-diagnostic-collector-progress-challenge-074-v1`, NUL and the complete LF document is echoed as nonce_sha256 in an otherwise unchanged signed DiagnosticRequest v1 Heartbeat. | Unknown/future/duplicate/noncanonical/oversized/partial documents refuse. Old clients cannot parse or downgrade this challenge; deploy a matched consumer. Byte inspection computes a digest, never an owner or acknowledgement. Exact owned nonce, original live inputs and exclusive challenge deadline are required; no regeneration, retry, recovered permission, durable progress, Starting or execution authority. |\n| Separate 0.74 original collector Reserved | `diagnostic-collector-reserved-074-v1` | opt-in Linux library only; no production consumer | Canonical LF JSON, at most 16384 bytes, in exclusive `diagnostic-collector-reserved-074.json` beside the retained controller attempt. Binds its full-byte SHA256, frozen receiver-observed initial state and separately signed collector envelope. Same original authority and fence remain owned; file and directory sync precede runtime acknowledgement. | Unknown/future/noncanonical/partial/mixed/extra context refuses. The bounded paired byte inspector does not verify signatures or reconstruct acknowledgement. Old readers reject the new inventory; no downgrade, overwrite, repair, deletion, legacy publisher conversion or adoption with unresolved context. Starting, outputs, execution, reboot and global nonce protection are not supplied. |\n| Separate 0.74 original controller attempt | `diagnostic-controller-attempt-074-v1` | opt-in Linux library only; no production consumer | Strict canonical LF JSON, at most 16384 bytes, in exclusively created `diagnostic-controller-attempt-074.json` under the original fixed intent leaf. Binds original Reserve request/hash, token digest (never token), build/config digests and original active policy/epoch/authority revision. The same original common fence and authority borrow remain held. | Unknown/future/duplicate/noncanonical/oversized/inconsistent fields refuse. Partial and complete retained context block fresh reservation; bytes never recover acknowledgement or start rights. Legacy intent bytes/names stay unchanged; controller documents cannot transfer into their publisher. No downgrade, deletion, repair or migration with unresolved context. Whole-host rollback and global nonce protection are outside this local contract. |\n", "| Artifact | Current Version | Writer | Reader Compatibility | Failure Mode |\n| --- | --- | --- | --- | --- |\n| Separate 0.74 collector Starting byte record | `diagnostic-collector-starting-074-v1` | opt-in Linux byte encoder only; no publisher or installed consumer | Canonical LF JSON, at most 16384 bytes; eventual fixed name `diagnostic-collector-starting-074.json`. Binds exact controller-attempt/collector-Reserved LF digests, canonical signed-consent LF digest and the identical signed Starting state; reuses strict prior-history and adjacent-transition grammar. | Unknown/future/duplicate/partial/noncanonical/oversized/mixed bindings refuse. Byte consistency does not verify signatures, inspect actual clocks/credentials or recover acknowledgement, replay ownership or start rights. Old two-file storage and legacy readers remain unchanged and reject a third record. No migration, overwrite, repair, publication, output, execution or cleanup authority. |\n| Separate 0.74 collector progress challenge | `diagnostic-collector-progress-challenge-074-v1` | opt-in Linux original-owner library only; no installed consumer | Canonical LF JSON, at most 4096 bytes; 32 system-random bytes, original controller-attempt/Reserved/canonical-state hashes, identity, revision 1 and original-kernel issuance BOOTTIME. SHA256 of `hydracache-diagnostic-collector-progress-challenge-074-v1`, NUL and the complete LF document is echoed as nonce_sha256 in an otherwise unchanged signed DiagnosticRequest v1 Heartbeat. | Unknown/future/duplicate/noncanonical/oversized/partial documents refuse. Old clients cannot parse or downgrade this challenge; deploy a matched consumer. Byte inspection computes a digest, never an owner or acknowledgement. Exact owned nonce, original live inputs and exclusive challenge deadline are required; no regeneration, retry, recovered permission, durable progress, Starting or execution authority. |\n| Separate 0.74 original collector Reserved | `diagnostic-collector-reserved-074-v1` | opt-in Linux library only; no production consumer | Canonical LF JSON, at most 16384 bytes, in exclusive `diagnostic-collector-reserved-074.json` beside the retained controller attempt. Binds its full-byte SHA256, frozen receiver-observed initial state and separately signed collector envelope. Same original authority and fence remain owned; file and directory sync precede runtime acknowledgement. | Unknown/future/noncanonical/partial/mixed/extra context refuses. The bounded paired byte inspector does not verify signatures or reconstruct acknowledgement. Old readers reject the new inventory; no downgrade, overwrite, repair, deletion, legacy publisher conversion or adoption with unresolved context. Starting, outputs, execution, reboot and global nonce protection are not supplied. |\n| Separate 0.74 original controller attempt | `diagnostic-controller-attempt-074-v1` | opt-in Linux library only; no production consumer | Strict canonical LF JSON, at most 16384 bytes, in exclusively created `diagnostic-controller-attempt-074.json` under the original fixed intent leaf. Binds original Reserve request/hash, token digest (never token), build/config digests and original active policy/epoch/authority revision. The same original common fence and authority borrow remain held. | Unknown/future/duplicate/noncanonical/oversized/inconsistent fields refuse. Partial and complete retained context block fresh reservation; bytes never recover acknowledgement or start rights. Legacy intent bytes/names stay unchanged; controller documents cannot transfer into their publisher. No downgrade, deletion, repair or migration with unresolved context. Whole-host rollback and global nonce protection are outside this local contract. |\n"),
    ("docs/testing/gated-test-registry.toml", "\n[gate.command.env]\n# END PRECONSENT LIVE GATE\n\n# BEGIN STARTING PUBLICATION API GATE\n[[gate]]\nid = \"cfg.hydracache-long-run-supervisor-074.diagnostic-collector-starting-publication\"\nkind = \"cfg_test_target\"\nsource = \"tools/long-run-supervisor-074/tests/diagnostic_collector_starting_publication.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"diagnostic_collector_starting_publication\"\ntest = \"\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original collector Starting intent only, no process or product workload\"\ntier = \"fast\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 120\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"rust\"\nstep = \"Release 0.74 Linux supervisor contracts\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--test\", \"diagnostic_collector_starting_publication\", \"--locked\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END STARTING PUBLICATION API GATE\n\n# BEGIN STARTING PUBLICATION FAULT GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.collector-starting-publication-boundaries-and-live-refusal\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"collector_starting_publication_boundaries_and_live_refusal\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original collector Starting intent only, no process or product workload\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"--release\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--lib\", \"--locked\", \"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::collector_starting_publication_boundaries_and_live_refusal\", \"--\", \"--ignored\", \"--exact\", \"--nocapture\", \"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END STARTING PUBLICATION FAULT GATE\n\n# BEGIN STARTING PUBLICATION CRASH GATE\n[[gate]]\nid = \"tool.hydracache-long-run-supervisor-074.collector-starting-process-crash-all-stages-and-lost-ack\"\nkind = \"external_tool\"\nsource = \"tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"hydracache_long_run_supervisor_074\"\ntest = \"collector_starting_process_crash_all_stages_and_lost_ack\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Original collector Starting intent only, no process or product workload\"\ntier = \"manual\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 1800\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"gated-proof-registry\"\nstep = \"Run registered gated proofs\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"--release\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--lib\", \"--locked\", \"diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::crash_tests::collector_starting_process_crash_all_stages_and_lost_ack\", \"--\", \"--ignored\", \"--exact\", \"--nocapture\", \"--test-threads=1\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END STARTING PUBLICATION CRASH GATE\n", "\n[gate.command.env]\n# END PRECONSENT LIVE GATE\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs", "use super::{open_at, stamp, AttemptStorage, Document, StorageAttemptError, CONTROLLER_NAME};\nuse std::os::fd::AsRawFd;\n\n#[path = \"diagnostic_collector_starting_storage.rs\"]\npub(crate) mod starting_storage;\n\npub(crate) const NAME: &str = \"diagnostic-collector-reserved-074.json\";\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum CollectorReservedStage {\n", "use super::{open_at, stamp, AttemptStorage, Document, StorageAttemptError, CONTROLLER_NAME};\nuse std::os::fd::AsRawFd;\n\npub(crate) const NAME: &str = \"diagnostic-collector-reserved-074.json\";\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum CollectorReservedStage {\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs", "    document: Option<Document>,\n    attempted: bool,\n    acknowledged: bool,\n    starting: Option<starting_storage::StartingPhase>,\n}\nimpl<'storage> CollectorReservationStorage<'storage> {\n    pub(crate) fn from_original(original: AttemptStorage<'storage>) -> Self {\n", "    document: Option<Document>,\n    attempted: bool,\n    acknowledged: bool,\n}\nimpl<'storage> CollectorReservationStorage<'storage> {\n    pub(crate) fn from_original(original: AttemptStorage<'storage>) -> Self {\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs", "            document: None,\n            attempted: false,\n            acknowledged: false,\n            starting: None,\n        }\n    }\n    pub(crate) fn is_refused(&self) -> bool {\n", "            document: None,\n            attempted: false,\n            acknowledged: false,\n        }\n    }\n    pub(crate) fn is_refused(&self) -> bool {\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs", "        if self.is_refused() || !self.acknowledged {\n            return Err(StorageAttemptError::Refused);\n        }\n        let result = if self.starting.is_some() {\n            self.inspect_starting()\n        } else {\n            self.inspect()\n        };\n        if result.is_err() {\n            self.latch_refusal();\n        }\n", "        if self.is_refused() || !self.acknowledged {\n            return Err(StorageAttemptError::Refused);\n        }\n        let result = self.inspect();\n        if result.is_err() {\n            self.latch_refusal();\n        }\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs", "use super::*;\n#[path = \"diagnostic_validation_frame.rs\"]\nmod validation_frame;\npub use validation_frame::starting;\npub use validation_frame::{OriginalFramedConsent, OriginalFramedConsentFailure};\n\nuse crate::diagnostic_start_consent::{\n", "use super::*;\n#[path = \"diagnostic_validation_frame.rs\"]\nmod validation_frame;\npub use validation_frame::{OriginalFramedConsent, OriginalFramedConsentFailure};\n\nuse crate::diagnostic_start_consent::{\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs", "//! Constant-depth orchestration; all original live leaf guards remain active.\nuse super::*;\n\n#[path = \"diagnostic_collector_starting_publication.rs\"]\npub mod starting;\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub(crate) enum ValidationFrameStep {\n    StorageBefore,\n", "//! Constant-depth orchestration; all original live leaf guards remain active.\nuse super::*;\n\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub(crate) enum ValidationFrameStep {\n    StorageBefore,\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs", "/// ```\n/// ```compile_fail\n/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;\n/// fn publish(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.publish_starting(b\"replacement\"); }\n/// ```\n/// ```compile_fail\n/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;\n", "/// ```\n/// ```compile_fail\n/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;\n/// fn publish(p: OriginalFramedConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.publish_starting(); }\n/// ```\n/// ```compile_fail\n/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::OriginalFramedConsent;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs", "#[path = \"diagnostic_validation_frame_budget_tests.rs\"]\nmod budget;\n\n#[path = \"diagnostic_collector_starting_publication_tests.rs\"]\nmod starting_tests;\n\n#[test]\nfn validation_frame_consuming_adapter_exists() {\n    fn probe(owner: OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {\n", "#[path = \"diagnostic_validation_frame_budget_tests.rs\"]\nmod budget;\n\n#[test]\nfn validation_frame_consuming_adapter_exists() {\n    fn probe(owner: OriginalStartConsent<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) {\n"),
    ("crates/xtask/tests/support/performance_contract_074_preconsent_frame.rs", "pub(super) fn normalize(path: &str, raw: &str) -> String {\n    let current = super::starting_publication::normalize(path, raw);\n    let raw = current.as_str();\n    let mut clean = raw.to_owned();", "pub(super) fn normalize(path: &str, raw: &str) -> String {\n    let mut clean = raw.to_owned();"),
    ("crates/xtask/tests/support/performance_contract_074_preconsent_frame.rs", "        assert_eq!(\n            super::starting_publication::normalize(p, &read(p)),\n            old(p),\n            \"out of scope: {p}\"\n        );", "        assert_eq!(read(p), old(p), \"out of scope: {p}\");"),
];
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::original_pipes::normalize(path, raw);
    let raw = current.as_str();
    let mut clean = raw.to_owned();
    for (p, new, old) in RESTORATIONS {
        if path == *p {
            assert_eq!(
                clean.matches(new).count(),
                1,
                "unapproved Starting publisher adapter: {path}"
            );
            clean = clean.replacen(new, old, 1);
        }
    }
    clean
}
#[test]
fn collector_starting_publication_preserves_every_original_predicate_and_deadline() {
    let read = |p: &str| {
        crate::runtime_io::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("77b3ad15:{p}")])
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
            "original predicate changed: {p}"
        );
        assert_ne!(
            normalize(p, &format!("{}\n// unapproved\n", read(p))),
            old(p)
        );
    }
    let p = "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs";
    assert!(std::panic::catch_unwind(|| normalize(
        p,
        &read(p).replace("starting_storage::StartingPhase", "UncheckedPhase")
    ))
    .is_err());
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_record.rs",
        "tools/long-run-supervisor-074/src/host_execution.rs",
        "tools/long-run-supervisor-074/src/server.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        // BEGIN FIXED MANAGER DEPENDENCY REFERENCE
        let current = read(p);
        let current = if matches!(
            p,
            "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs"
                | "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs"
        ) {
            crate::live_walk::normalize(p, &current)
        } else {
            current
        };
        let current = if matches!(
            p,
            "Cargo.lock"
                | "tools/long-run-supervisor-074/Cargo.toml"
                | "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs"
                | "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs"
        ) {
            super::original_start_consent::normalize(p, &current)
        } else {
            current
        };
        assert_eq!(current, old(p), "out of scope: {p}");
        // END FIXED MANAGER DEPENDENCY REFERENCE
    }
    // The rejected candidate remains the frozen publisher/control reference.
    let archived = read("docs/testing/performance/0.74/local-runs/collector-starting-publication-20261016/candidate-before-attribution.patch");
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs",
        "tools/long-run-supervisor-074/tests/diagnostic_collector_starting_publication.rs",
    ] {
        let header = format!("diff --git a/{p} b/{p}\n");
        let section = archived.split_once(&header).unwrap().1;
        let section = section.split("\ndiff --git ").next().unwrap();
        let mut expected = section
            .lines()
            .filter_map(|line| line.strip_prefix('+').filter(|_| !line.starts_with("+++")))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        if p.ends_with("diagnostic_collector_starting_publication_tests.rs") {
            expected = expected
                .replace(
                    "causal.prepare_start_consent(",
                    "causal.prepare_framed_consent(",
                )
                .replace(
                    "        .unwrap()\n        .into_validation_frame()\n        .unwrap()",
                    "        .unwrap()",
                );
        }
        assert_eq!(
            super::original_pipes::normalize(p, &read(p)),
            expected,
            "archived publisher, cuts or workload changed: {p}"
        );
    }
    let storage =
        read("tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs");
    let publisher =
        read("tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication.rs");
    for marker in [
        "libc::O_EXCL",
        "0o600",
        "sync_all()",
        "owned_write(&leaf, bytes, seal)",
        "phase.next != STAGES.len()",
        "phase.acknowledged = true",
        "controller.inspect(leaf)?",
        "reserved.inspect(leaf)?",
        "[true, true, phase.document.is_some()]",
    ] {
        assert!(storage.contains(marker), "{marker}");
    }
    for marker in [
        "self.original.revalidate()?",
        ".starting_step(at, &raw)?",
        "checkpoint(at)?",
        ".acknowledge_starting()?",
        "held.original.refuse()",
        "encode_collector_starting_record",
        "self.original.original.packet.bytes()",
    ] {
        assert!(publisher.contains(marker), "{marker}");
    }
    assert!(
        publisher.find(".acknowledge_starting()?").unwrap()
            < publisher.find("self.acknowledged = true").unwrap()
    );
    let runtime = publisher
        .lines()
        .filter(|line| !line.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for denied in [
        "std::process",
        "std::fs",
        "pub fn recover",
        "pub fn start_once",
        "O_TRUNC",
        "SystemTime",
        "SigningKey",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
    let crash =
        read("tools/long-run-supervisor-074/src/diagnostic_collector_starting_crash_tests.rs");
    for marker in [
        "libc::SYS_pidfd_send_signal",
        "Some(libc::SIGKILL)",
        "for repeat in 1..=3",
        "fresh_inspections=22",
        "sigkills=10",
        "assert_eq!(snapshot(&s.origin()), origin)",
        "inspect_empty_fence_at_for_test",
        "with_original_at(false, Some(&authority)",
        "let framed = accepted(original, fence, leaf, receiver, sender, model)",
    ] {
        assert!(
            crash
                .split_whitespace()
                .collect::<String>()
                .contains(&marker.split_whitespace().collect::<String>()),
            "{marker}"
        );
    }
}
