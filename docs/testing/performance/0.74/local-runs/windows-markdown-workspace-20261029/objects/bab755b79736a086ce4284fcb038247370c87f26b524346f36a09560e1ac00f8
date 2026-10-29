//! Complete old verifier remains a private differential oracle, never a runtime mode.
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

impl CheckedWorkerPolicy {
    fn revalidate_full_reference(
        &mut self,
        bytes: &[u8],
        current_trust: &WorkerPolicyTrust,
        asserted_context: &AssertedWorkerHost,
    ) -> Result<(), WorkerPolicyError> {
        if self.refused {
            return Err(WorkerPolicyError::Refused);
        }
        let result = (|| {
            if current_trust != &self.trust {
                return Err(WorkerPolicyError::Revoked);
            }
            if asserted_context != &self.context {
                return Err(WorkerPolicyError::Context);
            }
            if bytes.len() > MAX_SIGNED_WORKER_POLICY_BYTES {
                return Err(WorkerPolicyError::Document);
            }
            if sha256_hex(bytes) != self.envelope_sha256 {
                return Err(WorkerPolicyError::Drift);
            }
            let observed = verify_document(bytes, current_trust, asserted_context)?;
            if observed != self.original {
                return Err(WorkerPolicyError::Drift);
            }
            Ok(())
        })();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap();
    bytes.push(b'\n');
    bytes
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn policy() -> WorkerPolicy {
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
        supplementary_gids: vec![987, 988],
        machine_id: "a".repeat(32),
        boot_id: "11111111-2222-3333-4444-555555555555".into(),
        user_namespace: NamespaceIdentity {
            device: 4,
            inode: 111,
        },
        mount_namespace: NamespaceIdentity {
            device: 4,
            inode: 222,
        },
        passwd_sha256: "b".repeat(64),
        group_sha256: "c".repeat(64),
    }
}
fn context(p: &WorkerPolicy) -> AssertedWorkerHost {
    AssertedWorkerHost {
        machine_id: p.machine_id.clone(),
        boot_id: p.boot_id.clone(),
        user_namespace: p.user_namespace,
        mount_namespace: p.mount_namespace,
    }
}
fn key() -> SigningKey {
    SigningKey::from_bytes(&[74; 32])
}
fn envelope(p: &WorkerPolicy, signing_key: &SigningKey, domain: &[u8]) -> Vec<u8> {
    let mut message = domain.to_vec();
    message.push(0);
    message.extend(canonical(p));
    canonical(&SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy: p.clone(),
        signature_hex: hex(&signing_key.sign(&message).to_bytes()),
    })
}
fn trust(p: &WorkerPolicy) -> WorkerPolicyTrust {
    WorkerPolicyTrust::new(
        key().verifying_key(),
        &digest(&canonical(p)),
        p.policy_epoch,
    )
    .unwrap()
}

#[test]
fn worker_proof_reuse_verifies_once_and_retains_projection_integrity() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let (_, report) = crate::diagnostic_ipc::validation_trace::capture(|| {
        let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
        for _ in 0..8 {
            checked.revalidate(&bytes, &t, &host).unwrap();
        }
        checked.original.uid += 1;
        assert_eq!(
            checked.revalidate(&bytes, &t, &host),
            Err(WorkerPolicyError::Drift)
        );
        assert!(checked.is_refused());
    });
    assert_eq!(report.stages[0].worker_policy_calls, 1);
}
fn projection_mutation(p: &mut WorkerPolicy, which: usize) {
    match which % 16 {
        0 => p.schema_version.push('x'),
        1 => p.repository_id += 1,
        2 => p.purpose.push('x'),
        3 => p.account_source.push('x'),
        4 => p.account.push('x'),
        5 => p.group.push('x'),
        6 => p.policy_epoch += 1,
        7 => p.uid += 1,
        8 => p.gid += 1,
        9 => p.supplementary_gids.push(999),
        10 => p.machine_id.push('x'),
        11 => p.boot_id.push('x'),
        12 => p.user_namespace.inode += 1,
        13 => p.mount_namespace.inode += 1,
        14 => p.passwd_sha256.push('x'),
        _ => p.group_sha256.push('x'),
    }
}
fn pair_check(
    bytes: &[u8],
    t: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
    mutate: impl FnOnce(
        &mut CheckedWorkerPolicy,
        &mut CheckedWorkerPolicy,
        &mut Vec<u8>,
        &mut WorkerPolicyTrust,
        &mut AssertedWorkerHost,
    ),
) {
    let mut candidate = verify_worker_policy(bytes, t, host).unwrap();
    let mut reference = verify_worker_policy(bytes, t, host).unwrap();
    let mut input = bytes.to_vec();
    let mut current = t.clone();
    let mut context = host.clone();
    mutate(
        &mut candidate,
        &mut reference,
        &mut input,
        &mut current,
        &mut context,
    );
    let a = candidate.revalidate(&input, &current, &context);
    let b = reference.revalidate_full_reference(&input, &current, &context);
    assert_eq!(a, b);
    assert_eq!(candidate.is_refused(), reference.is_refused());
    // Original correct bytes cannot heal either owner after any first refusal.
    assert_eq!(
        candidate.revalidate(bytes, t, host),
        reference.revalidate_full_reference(bytes, t, host)
    );
    assert_eq!(candidate.is_refused(), reference.is_refused());
}
#[test]
fn worker_proof_reuse_reference_matches_every_retained_projection_field() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    for which in 0..16 {
        pair_check(&bytes, &t, &host, |a, b, _, _, _| {
            projection_mutation(&mut a.original, which);
            projection_mutation(&mut b.original, which);
        });
    }
}
#[test]
fn worker_proof_reuse_reference_matches_seeded_input_trust_context_and_priority() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let mut seed = 0x0740c0a1_u64;
    for case in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        pair_check(&bytes, &t, &host, |a, b, input, trust, context| {
            match case % 16 {
                0 => {}
                1 => input.clear(),
                2 => *input = vec![b' '; MAX_SIGNED_WORKER_POLICY_BYTES + 1],
                3 => {
                    let i = seed as usize % input.len();
                    input[i] ^= 1;
                }
                4 => {
                    input.pop();
                }
                5 => input.push(b' '),
                6 => trust.epoch += 1,
                7 => trust.key = SigningKey::from_bytes(&[75; 32]).verifying_key(),
                8 => trust.policy_sha256 = "d".repeat(64),
                9 => context.boot_id.push('x'),
                10 => context.machine_id.push('x'),
                11 => context.user_namespace.inode += 1,
                12 => context.mount_namespace.inode += 1,
                13 => {
                    a.refused = true;
                    b.refused = true;
                    input.clear();
                    trust.epoch += 1;
                }
                14 => {
                    trust.epoch += 1;
                    context.boot_id.push('x');
                    input.clear();
                }
                _ => {
                    context.boot_id.push('x');
                    input.clear();
                }
            }
        });
    }
    println!("worker_proof_reuse seed=0x0740c0a1 cases=128 projection_fields=16 original_error_order=true");
}
#[test]
fn worker_proof_reuse_independent_concurrent_owners_share_no_cached_authority() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let tasks: Vec<_> = (0..4)
        .map(|i| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let p = policy();
                let t = trust(&p);
                let host = context(&p);
                let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
                let mut checked = verify_worker_policy(&bytes, &t, &host).unwrap();
                barrier.wait();
                if i == 0 {
                    assert_eq!(
                        checked.revalidate(b"", &t, &host),
                        Err(WorkerPolicyError::Drift)
                    );
                    assert_eq!(
                        checked.revalidate(&bytes, &t, &host),
                        Err(WorkerPolicyError::Refused)
                    );
                } else {
                    for _ in 0..8 {
                        checked.revalidate(&bytes, &t, &host).unwrap();
                    }
                    assert!(!checked.is_refused());
                }
            })
        })
        .collect();
    for task in tasks {
        task.join().unwrap();
    }
}

#[test]
fn worker_proof_reuse_two_constructors_are_not_one_shared_admission() {
    let p = policy();
    let t = trust(&p);
    let host = context(&p);
    let bytes = envelope(&p, &key(), WORKER_POLICY_DOMAIN);
    let (_, report) = crate::diagnostic_ipc::validation_trace::capture(|| {
        let mut owners = [
            verify_worker_policy(&bytes, &t, &host).unwrap(),
            verify_worker_policy(&bytes, &t, &host).unwrap(),
        ];
        for owner in &mut owners {
            for _ in 0..8 {
                owner.revalidate(&bytes, &t, &host).unwrap();
            }
            assert!(!owner.is_refused());
        }
    });
    assert_eq!(report.stages[0].worker_policy_calls, 2);
}
