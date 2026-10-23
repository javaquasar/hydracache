use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::{
    collect, tests::Fixture,
};
use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage};
fn context() -> Context {
    Context {
        state: DiagnosticState {
            identity: DiagnosticIdentity {
                lease_id: "a".repeat(64),
                boot_id: "00000000-0000-4000-8000-000000000074".into(),
                binary_sha256: "b".repeat(64),
                build_provenance_sha256: "c".repeat(64),
            },
            revision: 2,
            stage: DiagnosticStage::Starting,
            completed_cells: 0,
            reserved_monotonic_ns: 1,
            last_observed_monotonic_ns: 3,
            controller_monotonic_ns: 2,
            cell_started_monotonic_ns: Some(3),
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: false,
            promotable: false,
            admission_allowed: false,
        },
        owner: [74, 75, 1000, 1000, 1000, 1000],
        origins: [1, 2, 3, 4],
        bounds: [300, 60, 10, 10],
        inventory: std::array::from_fn(|i| i as i32 + 20),
        custody: [40, 41],
        addressed: [42, 43],
        names: (":1.2".into(), ":1.3".into()),
        serial: NonZeroU32::new(74).unwrap(),
        root: RootBindingClaims {
            attempt_sha256: [1; 32],
            original_context_sha256: [2; 32],
            root_device: 1,
            root_inode: 2,
            deadline_boottime_ns: 10,
        },
        documents_sha256: std::array::from_fn(|i| format!("{i:064x}")),
        host: Image {
            identity: [1, 2, 64],
            sha256: "a".repeat(64),
        },
        launcher: Image {
            identity: [1, 3, 64],
            sha256: "b".repeat(64),
        },
        worker: Image {
            identity: [1, 4, 64],
            sha256: "c".repeat(64),
        },
        config: Image {
            identity: [99, 98, 64],
            sha256: "d".repeat(64),
        },
        streams: [[4, 6], [4, 7]],
        guardians: [(76, [5, 8]), (77, [5, 9])],
    }
}
fn legacy() -> LegacyContribution {
    LegacyContribution {
        config: [1, 88, 64],
        legacy: [1, 89, 256],
        stdin: [1, 90],
        channel: [1, 91],
        hash: [9; 32],
    }
}
#[test]
fn bootstrap_entry_factory_exact_original_fields_and_sealed_copy_not_source() {
    // The factory is tested with a modeled context/receipt, not a live permit.
    let f = Fixture::new();
    let census = collect(
        &f.prior(),
        f.original(),
        f.images.each_ref(),
        f.identities(),
    )
    .unwrap();
    let build = bootstrap_build::tests::Fixture::new();
    let docs = [build.policy_bytes(), build.receipt_bytes()];
    let c = context();
    let raw = expected(&c, &docs, [1, 92, 64], legacy(), &census).unwrap();
    let parsed = AssertedEntry::decode(&raw).unwrap();
    assert_eq!(parsed.owner, c.owner.map(|v| v as u64));
    assert_eq!(parsed.origins, c.origins);
    assert_eq!(parsed.bounds, c.bounds);
    assert_eq!(parsed.root, [1, 2, 10]);
    assert_eq!(parsed.serial, 74);
    assert_eq!(parsed.attempt, [1; 32]);
    assert_eq!(parsed.original_context, [2; 32]);
    assert_eq!(
        parsed.identities,
        [
            c.host.identity,
            c.launcher.identity,
            [1, 92, 64],
            c.worker.identity,
            [1, 88, 64],
            [1, 89, 256]
        ]
    );
    assert_ne!(parsed.identities[4], c.config.identity);
    assert_eq!(parsed.streams, [[1, 90], [4, 6], [4, 7], [1, 91]]);
    assert_eq!(parsed.guardians, [[76, 5, 8], [77, 5, 9]]);
    for (i, d) in c.documents_sha256.iter().enumerate() {
        assert_eq!(parsed.documents[i], hash(d).unwrap());
    }
    assert_eq!(
        parsed.documents[5],
        <[u8; 32]>::from(Sha256::digest(&docs[0]))
    );
    assert_eq!(
        parsed.documents[6],
        <[u8; 32]>::from(Sha256::digest(&docs[1]))
    );
    assert_eq!(
        parsed.names,
        [
            <[u8; 32]>::from(Sha256::digest(c.names.0.as_bytes())),
            <[u8; 32]>::from(Sha256::digest(c.names.1.as_bytes()))
        ]
    );
    assert_eq!(parsed.legacy_hash, [9; 32]);
    assert_eq!(parsed.census_hash, census.owner_fingerprint());
    assert_eq!(
        parsed.state_hash,
        <[u8; 32]>::from(Sha256::digest(crate::canonical_json(&c.state).unwrap()))
    );
    let mut changed = c.clone();
    changed.state.revision += 1;
    assert_ne!(
        expected(&changed, &docs, [1, 92, 64], legacy(), &census).unwrap(),
        raw
    );
    let mut changed = c.clone();
    changed.origins[0] += 1;
    assert_ne!(
        expected(&changed, &docs, [1, 92, 64], legacy(), &census).unwrap(),
        raw
    );
    let mut changed = c.clone();
    changed.root.deadline_boottime_ns += 1;
    assert!(expected(&changed, &docs, [1, 92, 64], legacy(), &census).is_err());
    let mut changed = c.clone();
    changed.documents_sha256[0] = "invalid".into();
    assert!(expected(&changed, &docs, [1, 92, 64], legacy(), &census).is_err());
}
