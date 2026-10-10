#![cfg(target_os = "linux")]
//! Synthetic operator pins over local observations, never real issuer enrollment.
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::{
    inspect_worker_context, WorkerContextError,
};
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::MetadataExt;

fn canonical(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    bytes.push(b'\n');
    bytes
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn actual_policy() -> WorkerPolicy {
    let ns = |name: &str| {
        let m = fs::metadata(format!("/proc/thread-self/ns/{name}")).unwrap();
        NamespaceIdentity {
            device: m.dev(),
            inode: m.ino(),
        }
    };
    WorkerPolicy {
        schema_version: WORKER_POLICY_SCHEMA.into(),
        repository_id: 1_217_101_761,
        purpose: "diagnostic-worker-only".into(),
        account_source: "local-files".into(),
        account: "hydracache-perf".into(),
        group: "hydracache-perf".into(),
        policy_epoch: 7,
        uid: 986,
        gid: 986,
        supplementary_gids: vec![],
        machine_id: fs::read_to_string("/etc/machine-id")
            .unwrap()
            .trim_end_matches('\n')
            .into(),
        boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .unwrap()
            .trim_end_matches('\n')
            .into(),
        user_namespace: ns("user"),
        mount_namespace: ns("mnt"),
        passwd_sha256: "b".repeat(64),
        group_sha256: "c".repeat(64),
    }
}
fn signed(p: &WorkerPolicy) -> (Vec<u8>, WorkerPolicyTrust, AssertedWorkerHost) {
    let key = SigningKey::from_bytes(&[74; 32]);
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(canonical(p));
    let bytes = canonical(&SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy: p.clone(),
        signature_hex: key
            .sign(&message)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    });
    let trust = WorkerPolicyTrust::new(key.verifying_key(), &digest(&canonical(p)), p.policy_epoch)
        .unwrap();
    let host = AssertedWorkerHost {
        machine_id: p.machine_id.clone(),
        boot_id: p.boot_id.clone(),
        user_namespace: p.user_namespace,
        mount_namespace: p.mount_namespace,
    };
    (bytes, trust, host)
}

#[test]
fn worker_context_actual_reader_roundtrip_keeps_original_policy_healthy() {
    let (bytes, trust, host) = signed(&actual_policy());
    let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
    {
        let mut read = inspect_worker_context(&mut policy, &bytes, &trust, &host).unwrap();
        for _ in 0..8 {
            read.revalidate(&bytes, &trust, &host).unwrap();
        }
        assert!(!read.is_refused());
    }
    policy.revalidate(&bytes, &trust, &host).unwrap();
}

#[test]
fn worker_context_valid_resigned_foreign_context_refuses_original_policy() {
    let actual = actual_policy();
    let changes: [fn(&mut WorkerPolicy); 5] = [
        |p| {
            p.machine_id = if p.machine_id == "a".repeat(32) {
                "b".repeat(32)
            } else {
                "a".repeat(32)
            }
        },
        |p| {
            p.boot_id = if p.boot_id.starts_with('a') {
                "bbbbbbbb-2222-3333-4444-555555555555"
            } else {
                "aaaaaaaa-2222-3333-4444-555555555555"
            }
            .into()
        },
        |p| p.user_namespace.inode += 1,
        |p| p.mount_namespace.device += 1,
        |p| std::mem::swap(&mut p.user_namespace, &mut p.mount_namespace),
    ];
    for change in changes {
        let mut foreign = actual.clone();
        change(&mut foreign);
        let (bytes, trust, host) = signed(&foreign);
        let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
        assert!(matches!(
            inspect_worker_context(&mut policy, &bytes, &trust, &host),
            Err(WorkerContextError::Drift)
        ));
        assert!(policy.is_refused());
        assert!(matches!(
            inspect_worker_context(&mut policy, &bytes, &trust, &host),
            Err(WorkerContextError::Refused)
        ));
    }
}

#[test]
fn worker_context_trust_and_envelope_failure_latch_through_drop() {
    let p = actual_policy();
    let (bytes, trust, host) = signed(&p);
    for changed_trust in [true, false] {
        let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
        {
            let mut read = inspect_worker_context(&mut policy, &bytes, &trust, &host).unwrap();
            let revoked = WorkerPolicyTrust::new(
                SigningKey::from_bytes(&[74; 32]).verifying_key(),
                &digest(&canonical(&p)),
                8,
            )
            .unwrap();
            let error = if changed_trust {
                read.revalidate(&bytes, &revoked, &host)
            } else {
                read.revalidate(b"{}\n", &trust, &host)
            };
            assert!(matches!(error, Err(WorkerContextError::Policy(_))));
            assert!(read.is_refused());
            assert!(matches!(
                read.revalidate(&bytes, &trust, &host),
                Err(WorkerContextError::Refused)
            ));
        }
        assert!(policy.is_refused());
    }
}

#[test]
fn worker_context_seeded_valid_namespace_mutations_cannot_choose_new_pins() {
    let seed = 0x7542026_u64;
    eprintln!("worker context mutation seed={seed:#x}");
    let actual = actual_policy();
    let mut rng = seed;
    for _ in 0..64 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut p = actual.clone();
        if rng & 1 == 0 {
            p.user_namespace.inode += 1 + (rng % 1000);
        } else {
            p.mount_namespace.inode += 1 + (rng % 1000);
        }
        let (bytes, trust, host) = signed(&p);
        let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
        assert!(matches!(
            inspect_worker_context(&mut policy, &bytes, &trust, &host),
            Err(WorkerContextError::Drift)
        ));
        assert!(policy.is_refused());
    }
}

#[test]
fn worker_context_independent_concurrent_readers_keep_thread_local_refusal() {
    std::thread::scope(|scope| {
        for index in 0..4 {
            scope.spawn(move || {
                let (bytes, trust, host) = signed(&actual_policy());
                let mut policy = verify_worker_policy(&bytes, &trust, &host).unwrap();
                let mut read = inspect_worker_context(&mut policy, &bytes, &trust, &host).unwrap();
                if index == 0 {
                    assert!(read.revalidate(b"bad", &trust, &host).is_err());
                } else {
                    for _ in 0..4 {
                        read.revalidate(&bytes, &trust, &host).unwrap();
                    }
                }
                assert_eq!(read.is_refused(), index == 0);
            });
        }
    });
}
