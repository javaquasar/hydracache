//! Synthetic configuration/keys and actual local kernel inputs, never enrollment.
#[path = "consuming_held_bridge.rs"]
mod consuming_held_bridge;
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

fn uid() -> u32 {
    unsafe { libc::geteuid() }
}
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = canonical_json(value).unwrap();
    bytes.push(b'\n');
    bytes
}
struct Fixture {
    root: tempfile::TempDir,
    document: Document,
    selection: Selection,
    held_policy: Vec<u8>,
    worker_policy: Vec<u8>,
}
impl Fixture {
    fn new(controller: VerifyingKey, worker_policy_sha256: String, epoch: u64) -> Self {
        let worker = SigningKey::from_bytes(&[80; 32]).verifying_key();
        let issuer = SigningKey::from_bytes(&[81; 32]).verifying_key();
        let builder = SigningKey::from_bytes(&[82; 32]).verifying_key();
        let worker_policy =
            crate::diagnostic_builder::policy_bytes(&crate::diagnostic_builder::BuilderPolicy {
                schema_version: crate::diagnostic_builder::POLICY_SCHEMA.into(),
                repository_id: crate::diagnostic_builder::REPOSITORY_ID,
                builder_id: crate::diagnostic_builder::BUILDER_ID.into(),
                builder_key_hex: crate::hex(worker.as_bytes()),
                controller_key_hex: crate::hex(controller.as_bytes()),
            })
            .unwrap();
        let held_policy = held::document_bytes(&held::Policy {
            schema_version: held::POLICY_SCHEMA.into(),
            purpose: held::PURPOSE.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            builder_id: held::BUILDER_ID.into(),
            builder_key_hex: crate::hex(builder.as_bytes()),
            controller_key_hex: crate::hex(controller.as_bytes()),
            authority_epoch: epoch,
            source: held::Source {
                commit: "a".repeat(40),
                tree: "b".repeat(40),
                root_lock: crate::diagnostic_artifacts::ArtifactDigest::of(b"synthetic-lock\n"),
            },
        })
        .unwrap();
        let selection = Selection {
            controller,
            worker_builder: worker,
            worker_issuer: issuer,
            worker_builder_policy_sha256: crate::sha256_hex(&worker_policy),
        };
        let document = Document {
            schema_version: TRUST_SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            controller_key_hex: crate::hex(controller.as_bytes()),
            held_builder_key_hex: crate::hex(builder.as_bytes()),
            held_policy_sha256: crate::sha256_hex(&held_policy),
            worker_builder_policy_sha256: selection.worker_builder_policy_sha256.clone(),
            worker_policy_sha256,
            policy_epoch: epoch,
        };
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.path().join("etc")).unwrap();
        fs::create_dir(root.path().join("etc/hydracache-perf")).unwrap();
        let value = Self {
            root,
            document,
            selection,
            held_policy,
            worker_policy,
        };
        value.write();
        value
    }
    fn simple() -> Self {
        Self::new(
            SigningKey::from_bytes(&[76; 32]).verifying_key(),
            "c".repeat(64),
            7,
        )
    }
    fn path(&self) -> PathBuf {
        self.root
            .path()
            .join("etc/hydracache-perf")
            .join(TRUST_FILE)
    }
    fn write(&self) {
        fs::write(self.path(), line(&self.document)).unwrap();
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn input(&self) -> HeldTrustInput {
        HeldTrustInput {
            source: HeldSource::open(self.root.path(), uid(), self.selection.clone()).unwrap(),
            refused: false,
            _original_thread: PhantomData,
        }
    }
}
#[test]
fn held_trust_source_retains_independent_selection_and_revalidates() {
    let f = Fixture::simple();
    let mut source = f.input();
    for _ in 0..3 {
        source.revalidate().unwrap();
    }
    assert!(!source.is_refused());
}

#[test]
fn strict_fields_missing_null_duplicate_and_unknown_refuse() {
    let f = Fixture::simple();
    let value = serde_json::to_value(&f.document).unwrap();
    let raw = line(&f.document);
    for field in value.as_object().unwrap().keys() {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        let mut null = value.clone();
        null[field] = serde_json::Value::Null;
        let duplicate = format!(
            "{{\"{field}\":{},{}",
            value[field],
            std::str::from_utf8(&raw).unwrap().trim_start_matches('{')
        );
        for bad in [line(&missing), line(&null), duplicate.into_bytes()] {
            assert!(decode(&bad, &f.selection).is_err(), "{field}");
        }
    }
    let mut extra = value;
    extra["future"] = serde_json::json!(true);
    assert!(decode(&line(&extra), &f.selection).is_err());
}
#[test]
fn schema_repository_pins_epoch_and_controller_are_not_caller_selected() {
    let f = Fixture::simple();
    for (field, value) in [
        ("schema_version", serde_json::json!("future")),
        ("repository_id", serde_json::json!(1)),
        (
            "controller_key_hex",
            serde_json::json!(crate::hex(f.selection.worker_builder.as_bytes())),
        ),
        (
            "worker_builder_policy_sha256",
            serde_json::json!("d".repeat(64)),
        ),
        ("policy_epoch", serde_json::json!(0)),
        ("held_policy_sha256", serde_json::json!("G".repeat(64))),
        ("worker_policy_sha256", serde_json::json!("c".repeat(63))),
    ] {
        let mut doc = serde_json::to_value(&f.document).unwrap();
        doc[field] = value;
        assert!(decode(&line(&doc), &f.selection).is_err(), "{field}");
    }
    assert!(decode(&line(&f.document), &Selection::reviewed().unwrap()).is_err());
}
#[test]
fn held_key_is_nonweak_and_distinct_from_all_three_old_keys() {
    let mut f = Fixture::simple();
    for key in [
        f.selection.controller,
        f.selection.worker_builder,
        f.selection.worker_issuer,
    ] {
        f.document.held_builder_key_hex = crate::hex(key.as_bytes());
        assert!(decode(&line(&f.document), &f.selection).is_err());
    }
    f.document.held_builder_key_hex = "00".repeat(32);
    assert!(decode(&line(&f.document), &f.selection).is_err());
    for bad in ["ff".repeat(31), "Z".repeat(64)] {
        f.document.held_builder_key_hex = bad;
        assert!(decode(&line(&f.document), &f.selection).is_err());
    }
}
#[test]
fn canonical_newline_and_bounded_document_are_required() {
    let f = Fixture::simple();
    let raw = line(&f.document);
    for bad in [
        Vec::new(),
        vec![b' '; MAX_TRUST_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.clone(), vec![b'\n']].concat(),
        serde_json::to_vec_pretty(&f.document).unwrap(),
    ] {
        assert!(decode(&bad, &f.selection).is_err());
    }
}
#[test]
fn unsafe_leaf_modes_and_hardlinks_refuse() {
    for mode in [0o777, 0o660, 0o620, 0o601, 0o700, 0o200] {
        let f = Fixture::simple();
        fs::set_permissions(f.path(), fs::Permissions::from_mode(mode)).unwrap();
        assert!(
            HeldSource::open(f.root.path(), uid(), f.selection.clone()).is_err(),
            "{mode:o}"
        );
    }
    let f = Fixture::simple();
    fs::hard_link(f.path(), f.root.path().join("alias")).unwrap();
    assert!(HeldSource::open(f.root.path(), uid(), f.selection.clone()).is_err());
}
#[test]
fn wrong_owner_relative_root_empty_and_oversize_sources_refuse() {
    let f = Fixture::simple();
    assert!(HeldSource::open(f.root.path(), uid() + 1, f.selection.clone()).is_err());
    assert!(HeldSource::open(Path::new("relative"), uid(), f.selection.clone()).is_err());
    for raw in [Vec::new(), vec![b' '; MAX_TRUST_BYTES + 1]] {
        fs::write(f.path(), raw).unwrap();
        assert!(HeldSource::open(f.root.path(), uid(), f.selection.clone()).is_err());
    }
}
#[test]
fn leaf_symlink_directory_and_fifo_refuse_without_blocking() {
    for case in 0..3 {
        let f = Fixture::simple();
        let path = f.path();
        fs::rename(&path, f.root.path().join("saved")).unwrap();
        match case {
            0 => symlink(f.root.path().join("saved"), &path).unwrap(),
            1 => fs::create_dir(&path).unwrap(),
            _ => {
                use std::os::unix::ffi::OsStrExt;
                let path = CString::new(path.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
            }
        }
        assert!(HeldSource::open(f.root.path(), uid(), f.selection.clone()).is_err());
    }
}
#[test]
fn named_root_and_ancestor_symlinks_refuse() {
    let f = Fixture::simple();
    let alias = f.root.path().join("alias");
    symlink(f.root.path(), &alias).unwrap();
    assert!(HeldSource::open(&alias, uid(), f.selection.clone()).is_err());
    let dir = f.root.path().join("etc/hydracache-perf");
    let saved = f.root.path().join("saved");
    fs::rename(&dir, &saved).unwrap();
    symlink(&saved, &dir).unwrap();
    assert!(HeldSource::open(f.root.path(), uid(), f.selection.clone()).is_err());
}
#[test]
fn same_bytes_leaf_replacement_is_sticky_even_after_original_restore() {
    let f = Fixture::simple();
    let mut input = f.input();
    let saved = f.root.path().join("saved");
    fs::rename(f.path(), &saved).unwrap();
    f.write();
    assert!(input.revalidate().is_err());
    fs::remove_file(f.path()).unwrap();
    fs::rename(saved, f.path()).unwrap();
    assert!(matches!(input.revalidate(), Err(HeldTrustError::Refused)));
}
#[test]
fn same_inode_mutation_and_metadata_change_refuse_stickily() {
    for case in 0..2 {
        let f = Fixture::simple();
        let mut input = f.input();
        if case == 0 {
            let mut raw = line(&f.document);
            raw[3] ^= 1;
            fs::write(f.path(), raw).unwrap();
        } else {
            fs::set_permissions(f.path(), fs::Permissions::from_mode(0o400)).unwrap();
        }
        assert!(input.revalidate().is_err());
        fs::set_permissions(f.path(), fs::Permissions::from_mode(0o600)).unwrap();
        f.write();
        assert!(input.revalidate().is_err());
        assert!(input.is_refused());
    }
}
#[test]
fn same_bytes_ancestor_and_root_replacements_refuse() {
    for case in 0..2 {
        let f = Fixture::simple();
        let mut input = f.input();
        if case == 0 {
            let path = f.root.path().join("etc");
            fs::rename(&path, f.root.path().join("old-etc")).unwrap();
            fs::create_dir(&path).unwrap();
            fs::create_dir(path.join("hydracache-perf")).unwrap();
            f.write();
        } else {
            let container = tempfile::tempdir().unwrap();
            let saved = container.path().join("old-root");
            fs::rename(f.root.path(), &saved).unwrap();
            fs::create_dir(f.root.path()).unwrap();
            fs::set_permissions(f.root.path(), fs::Permissions::from_mode(0o700)).unwrap();
            fs::create_dir(f.root.path().join("etc")).unwrap();
            fs::create_dir(f.root.path().join("etc/hydracache-perf")).unwrap();
            f.write();
            assert!(input.revalidate().is_err());
            continue;
        }
        assert!(input.revalidate().is_err());
    }
}
#[test]
fn retained_projection_drift_refuses_instead_of_repairing_itself() {
    let f = Fixture::simple();
    let mut input = f.input();
    input.source.document.policy_epoch += 1;
    assert!(input.revalidate().is_err());
    assert!(input.is_refused());
}
#[test]
fn seeded_byte_mutations_cannot_select_different_trust() {
    let f = Fixture::simple();
    let raw = line(&f.document);
    let mut seed = 0x0074_7a57u64;
    eprintln!("held_runtime_trust seed=0x00747a57 corruptions=256");
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bad = raw.clone();
        let at = seed as usize % bad.len();
        bad[at] ^= 0x80;
        assert!(decode(&bad, &f.selection).is_err());
    }
}
#[test]
fn independent_thread_local_sources_do_not_share_refusal() {
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let f = Fixture::simple();
                let mut input = f.input();
                input.revalidate().unwrap();
                let g = Fixture::simple();
                let mut other = g.input();
                fs::write(g.path(), b"bad").unwrap();
                assert!(other.revalidate().is_err());
                input.revalidate().unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}
#[test]
fn production_opener_requires_root_before_accessing_fixed_configuration() {
    assert_ne!(uid(), 0, "this local fixture must run unprivileged");
    assert!(matches!(
        open_fixed_held_runtime_trust(),
        Err(HeldTrustError::Input(TrustInputError::Invalid))
    ));
}

fn with_original(case: impl FnOnce(&mut SignedControllerInput<'_, '_, '_, '_, '_>, &Fixture)) {
    let model = super::super::tests::Model::new();
    let bytes = model.sign(model.request(), &model.key);
    let (_worker_root, _authority_root, mut ws, mut authority, worker_bytes) =
        super::super::super::tests::controller_worker_fixture();
    let mut packet = super::super::super::tests::kernel_packet_bytes(&bytes);
    let mut worker = ws
        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
        .unwrap();
    let mut controller = model.input();
    let mut original = controller.authenticate_controller(&mut worker).unwrap();
    let observed = original.worker.controller_authority_binding().unwrap();
    let f = Fixture::new(model.key.verifying_key(), observed.0, observed.1);
    case(&mut original, &f);
}
#[test]
fn original_guard_reobserves_real_kernel_controller_and_active_authority() {
    with_original(|original, f| {
        let mut input = f.input();
        let mut guard = input
            .bind_original(original, &f.held_policy, &f.worker_policy)
            .unwrap();
        for _ in 0..3 {
            guard.revalidate().unwrap();
        }
        assert!(!guard.is_refused());
    });
}
#[test]
fn original_guard_poison_propagates_from_configuration_to_original_chain() {
    with_original(|original, f| {
        let mut input = f.input();
        {
            let mut guard = input
                .bind_original(original, &f.held_policy, &f.worker_policy)
                .unwrap();
            fs::write(f.path(), b"bad").unwrap();
            assert!(guard.revalidate().is_err());
            assert!(guard.is_refused());
        }
        assert!(original.is_refused());
        assert!(input.is_refused());
    });
}
#[test]
fn original_guard_requires_actual_authority_policy_and_epoch() {
    for case in 0..2 {
        with_original(|original, f| {
            let mut value: Document = serde_json::from_slice(&line(&f.document)).unwrap();
            if case == 0 {
                value.policy_epoch += 1;
            } else {
                value.worker_policy_sha256 = "d".repeat(64);
            }
            fs::write(f.path(), line(&value)).unwrap();
            let mut input = f.input();
            assert!(input
                .bind_original(original, &f.held_policy, &f.worker_policy)
                .is_err());
            assert!(input.is_refused());
            assert!(original.is_refused());
        });
    }
}
#[test]
fn original_guard_requires_independently_pinned_held_and_old_builder_documents() {
    for case in 0..3 {
        with_original(|original, f| {
            let mut input = f.input();
            let mut held = f.held_policy.clone();
            let mut worker = f.worker_policy.clone();
            match case {
                0 => held[3] ^= 1,
                1 => worker[3] ^= 1,
                _ => held = vec![0; crate::diagnostic_builder::MAX_POLICY_BYTES + 1],
            };
            assert!(input.bind_original(original, &held, &worker).is_err());
            assert!(input.is_refused());
            assert!(original.is_refused());
        });
    }
}
#[test]
fn actual_original_packet_namespace_and_policy_faults_poison_both_borrows() {
    for case in 0..3 {
        with_original(|original, f| {
            let mut input = f.input();
            {
                let mut guard = input
                    .bind_original(original, &f.held_policy, &f.worker_policy)
                    .unwrap();
                guard.original.worker.simulate_owned_fault_for_test(case);
                assert!(guard.revalidate().is_err());
                assert!(guard.is_refused());
            }
            assert!(input.is_refused());
            assert!(original.is_refused());
        });
    }
}
#[test]
fn retained_actual_revision_snapshot_and_request_drift_cannot_be_repaired() {
    for case in 0..3 {
        with_original(|original, f| {
            let mut input = f.input();
            let mut guard = input
                .bind_original(original, &f.held_policy, &f.worker_policy)
                .unwrap();
            match case {
                0 => guard.observed.authority.2 += 1,
                1 => guard.observed.authority.3 = "f".repeat(64),
                _ => guard.observed.request_sha256 = "f".repeat(64),
            };
            assert!(guard.revalidate().is_err());
            assert!(guard.is_refused());
        });
    }
}

// Signed synthetic build/placement documents exercise the early join only.
// Nothing here compiles or executes the represented binaries.
fn binding_pair(f: &Fixture, request: &DiagnosticRequest, case: u8) -> (Vec<u8>, Vec<u8>) {
    use crate::diagnostic_artifacts::ArtifactDigest;
    use crate::diagnostic_two_phase_launch::{
        self as launch, held_placement as placement, root_enrollment,
    };
    let elf = |tag| {
        let mut raw = vec![0; 128];
        raw[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        raw[16] = 2;
        raw[18] = 62;
        raw[20] = 1;
        raw[32] = 64;
        raw[52] = 64;
        raw[54] = 56;
        raw[56] = 1;
        raw[127] = tag;
        raw
    };
    let host = elf(1);
    let launcher = elf(2);
    let lock = b"synthetic-lock\n";
    let mut log = Vec::new();
    for (name, source) in [
        (held::HOST, "main.rs"),
        (held::LAUNCHER, "bin/diagnostic_held_launcher.rs"),
    ] {
        log.extend(line(&json!({"reason":"compiler-artifact","package_id":"path+file:///isolated/tools/long-run-supervisor-074#0.0.0","target":{"name":name,"kind":["bin"],"crate_types":["bin"],"src_path":format!("/isolated/tools/long-run-supervisor-074/src/{source}")},"profile":{"opt_level":"3","debug_assertions":false,"test":false},"features":[],"executable":format!("/target/x86_64-unknown-linux-gnu/release/{name}"),"fresh":false})));
    }
    log.extend(line(&json!({"reason":"build-finished","success":true})));
    let policy = held::load_policy(
        &f.held_policy,
        &f.document.held_policy_sha256,
        &held::ExpectedTrust {
            controller_key: &f.selection.controller,
            authority_epoch: f.document.policy_epoch,
            worker_policy: &f.worker_policy,
            worker_policy_sha256: &f.selection.worker_builder_policy_sha256,
        },
    )
    .unwrap();
    let receipt = held::document_bytes(
        &held::sign_build(
            &policy,
            &SigningKey::from_bytes(&[82; 32]),
            &held::Observation {
                source_commit_before: "a".repeat(40),
                source_commit_after: "a".repeat(40),
                source_tree_before: "b".repeat(40),
                source_tree_after: "b".repeat(40),
                source_clean_before: true,
                source_clean_after: true,
                isolated_build: true,
                rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
                cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
                build_command: held::BUILD_COMMAND.iter().map(|s| (*s).into()).collect(),
            },
            &held::Contents {
                original_host: &host,
                held_launcher: &launcher,
                root_lock: lock,
                build_log: &log,
            },
        )
        .unwrap(),
    )
    .unwrap();
    let mut consent = json!({
        "schema_version":crate::diagnostic_start_consent::BODY_SCHEMA,"action":"start-cell-one",
        "consent_request_id":"1".repeat(64),"reservation_request_id":request.request_id,
        "controller_attempt_sha256":"3".repeat(64),"collector_reserved_sha256":"4".repeat(64),
        "reserved_state_sha256":"5".repeat(64),"challenge_nonce_sha256":"6".repeat(64),
        "authenticated_response_sha256":"7".repeat(64),"worker_policy_sha256":f.document.worker_policy_sha256,
        "worker_policy_epoch":f.document.policy_epoch,"collector_unit_sha256":"9".repeat(64),
        "controller":request.controller,"source_commit":request.source_commit,"preset":request.preset,
        "starting_state":{"identity":request.identity,"revision":2,"stage":"starting","completed_cells":0,"reserved_monotonic_ns":1,"controller_monotonic_ns":3,"last_observed_monotonic_ns":4,"cell_started_monotonic_ns":4,"cgroup_inode":null,"reason":null,"cleanup_confirmed":false,"promotable":false,"admission_allowed":false},
        "issued_at_unix_seconds":request.issued_at_unix_seconds,"expires_at_unix_seconds":request.expires_at_unix_seconds
    });
    match case {
        1 => consent["reservation_request_id"] = json!("2".repeat(64)),
        2 => consent["controller"]["run_id"] = json!(78),
        3 => consent["starting_state"]["identity"]["lease_id"] = json!("f".repeat(64)),
        4 => consent["worker_policy_sha256"] = json!("8".repeat(64)),
        _ => {}
    }
    let consent = serde_json::from_value(consent).unwrap();
    let artifacts = launch::LaunchArtifacts {
        launcher_sha256: ArtifactDigest::of(&launcher).sha256,
        watchdog_sha256: ArtifactDigest::of(&host).sha256,
        collector_sha256: ArtifactDigest::of(&host).sha256,
    };
    let launch = launch::LaunchBody {
        schema_version: launch::BODY_SCHEMA.into(),
        action: "prepare-held-cell-one".into(),
        requirements_sha256: crate::sha256_hex(
            &launch::launch_requirements(&consent, &artifacts).unwrap(),
        ),
        consent_claims: consent,
        artifacts,
    };
    let root = root_enrollment::RootBindingClaims {
        attempt_sha256: Sha256::digest(
            serde_json::to_vec(&(
                "hc074-original-root-attempt-v1",
                &launch.consent_claims.starting_state.identity,
            ))
            .unwrap(),
        )
        .into(),
        original_context_sha256: [75; 32],
        root_device: 3,
        root_inode: 6,
        deadline_boottime_ns: 10_000_000_002,
    };
    let body = placement::PlacementBody {
        schema_version: placement::BODY_SCHEMA.into(),
        action: "project-held-cell-one".into(),
        root_enrollment_hex: crate::hex(
            &root_enrollment::PacketClaims {
                kind: root_enrollment::Kind::Enroll,
                binding: root,
            }
            .encode()
            .unwrap(),
        ),
        requirements_sha256: crate::sha256_hex(
            &placement::placement_requirements(&launch, &root).unwrap(),
        ),
        launch_claims: launch,
    };
    let controller = SigningKey::from_bytes(&[76; 32]);
    let signature_hex = crate::hex(
        &controller
            .sign(&placement::placement_signing_message(&body).unwrap())
            .to_bytes(),
    );
    let body = role_binding::Body {
        schema_version: role_binding::BODY_SCHEMA.into(),
        action: "bind-held-build-to-placement".into(),
        placement: placement::SignedPlacement {
            schema_version: placement::ENVELOPE_SCHEMA.into(),
            placement: body,
            signature_hex,
        },
        held_policy_sha256: f.document.held_policy_sha256.clone(),
        held_receipt_sha256: crate::sha256_hex(&receipt),
        authority_epoch: f.document.policy_epoch,
    };
    let signature_hex = crate::hex(
        &controller
            .sign(&role_binding::signing_message(&body, &policy, &receipt).unwrap())
            .to_bytes(),
    );
    (
        line(&role_binding::SignedBinding {
            schema_version: role_binding::ENVELOPE_SCHEMA.into(),
            binding: body,
            signature_hex,
        }),
        receipt,
    )
}
#[test]
fn valid_v4_documents_match_the_same_original_request_without_admitting_start() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let mut input = f.input();
        let mut guard = input
            .bind_original(original, &f.held_policy, &f.worker_policy)
            .unwrap();
        guard.inspect_request_binding(&raw, &receipt).unwrap();
        guard.revalidate().unwrap();
        assert!(!guard.is_refused());
    });
}
#[test]
fn validly_signed_v4_for_other_request_controller_identity_or_policy_poisons_join() {
    for case in 1..5 {
        with_original(|original, f| {
            let (raw, receipt) = binding_pair(f, &original.request, case);
            let mut input = f.input();
            let mut guard = input
                .bind_original(original, &f.held_policy, &f.worker_policy)
                .unwrap();
            let policy = guard.checked_policy().unwrap();
            role_binding::inspect_signature(&raw, &policy, &receipt).unwrap();
            assert!(matches!(
                guard.inspect_request_binding(&raw, &receipt),
                Err(HeldTrustError::ContextMismatch)
            ));
            assert!(guard.is_refused());
        });
    }
}
#[test]
fn malformed_v4_poisons_original_instead_of_returning_a_static_receipt() {
    with_original(|original, f| {
        let mut input = f.input();
        let mut guard = input
            .bind_original(original, &f.held_policy, &f.worker_policy)
            .unwrap();
        assert!(guard.inspect_request_binding(b"{}", b"{}").is_err());
        assert!(guard.is_refused());
    });
}
#[test]
fn valid_pin_with_different_root_selected_held_key_refuses() {
    with_original(|original, f| {
        let mut doc: Document = serde_json::from_slice(&line(&f.document)).unwrap();
        doc.held_builder_key_hex =
            crate::hex(SigningKey::from_bytes(&[83; 32]).verifying_key().as_bytes());
        fs::write(f.path(), line(&doc)).unwrap();
        let mut input = f.input();
        assert!(matches!(
            input.bind_original(original, &f.held_policy, &f.worker_policy),
            Err(HeldTrustError::ContextMismatch)
        ));
        assert!(original.is_refused());
    });
}
