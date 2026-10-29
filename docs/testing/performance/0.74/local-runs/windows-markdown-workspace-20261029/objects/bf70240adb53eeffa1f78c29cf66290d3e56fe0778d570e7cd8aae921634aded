//! Public historical builder signature; synthetic principals are not host admission.
use super::*;
use crate::diagnostic_ipc::DiagnosticRequest;
use crate::diagnostic_worker_policy::trusted_input::controller_input::tests::Model;
use crate::diagnostic_worker_policy::trusted_input::tests::{
    controller_worker_fixture, kernel_packet_bytes,
};

const POLICY: &[u8] =
    include_bytes!("../../../docs/testing/performance/0.74/diagnostic-builder-public-policy.json");
const RECEIPT: &[u8] = include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
const BINARY: &str = "bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101";

fn exercise(
    policy: &[u8],
    receipt: &[u8],
    change: impl FnOnce(&mut DiagnosticRequest),
    inspect: impl FnOnce(&mut ControllerBuildInput<'_, '_, '_, '_, '_, '_>, &Model),
    accepted: bool,
    refused: bool,
) {
    let model = Model::new();
    let mut request = model.request();
    request.identity.binary_sha256 = BINARY.into();
    request.identity.build_provenance_sha256 = crate::sha256_hex(RECEIPT);
    change(&mut request);
    let bytes = model.sign(request, &model.key);
    let (_root, _authority_root, mut source, mut authority, worker_bytes) =
        controller_worker_fixture();
    let mut packet = kernel_packet_bytes(&bytes);
    let mut worker = source
        .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
        .unwrap();
    let mut controller = model.input();
    {
        let mut original = controller.authenticate_controller(&mut worker).unwrap();
        let mut caller_policy = policy.to_vec();
        let mut caller_receipt = receipt.to_vec();
        match original.bind_reviewed_build(&caller_policy, &caller_receipt) {
            Ok(mut guard) => {
                assert!(accepted);
                // Inputs are owned copies, not caller-borrowed refresh material.
                caller_policy.fill(0);
                caller_receipt.fill(0);
                guard.revalidate().unwrap();
                inspect(&mut guard, &model);
                assert_eq!(guard.is_refused(), refused);
            }
            Err(_) => assert!(!accepted && refused),
        }
        assert_eq!(original.is_refused(), refused);
        if refused {
            assert!(original.bind_reviewed_build(POLICY, RECEIPT).is_err());
        } else {
            original.revalidate().unwrap();
        }
    }
    assert_eq!(controller.is_refused(), refused);
    assert_eq!(worker.is_refused(), refused);
    drop(worker);
    assert_eq!(source.is_refused(), refused);
    assert_eq!(authority.is_failed(), refused);
    assert_eq!(packet.is_refused(), refused);
}

#[test]
fn controller_build_roundtrip_owned_bytes_rechecks_and_healthy_drop() {
    exercise(
        POLICY,
        RECEIPT,
        |_| (),
        |guard, _| {
            assert_eq!(guard.build.identity().binary_sha256, BINARY);
            assert_eq!(
                guard.build.statement().repository_id,
                crate::diagnostic_builder::REPOSITORY_ID
            );
            assert_eq!(
                guard.build.statement().source_commit,
                crate::diagnostic_lease::SOURCE_COMMIT
            );
            for _ in 0..3 {
                guard.revalidate().unwrap();
            }
        },
        true,
        false,
    );
}

#[test]
fn controller_build_wrong_original_identity_refuses_whole_chain() {
    for binary in [true, false] {
        exercise(
            POLICY,
            RECEIPT,
            |request| {
                if binary {
                    request.identity.binary_sha256 = "0".repeat(64);
                } else {
                    request.identity.build_provenance_sha256 = "0".repeat(64);
                }
            },
            |_, _| unreachable!(),
            false,
            true,
        );
    }
}

#[test]
fn controller_build_invalid_policy_receipt_and_budgets_cross_latch() {
    let mut policy = POLICY.to_vec();
    policy[10] ^= 1;
    let mut receipt = RECEIPT.to_vec();
    receipt[10] ^= 1;
    for (policy, receipt) in [
        (vec![], RECEIPT.to_vec()),
        (policy, RECEIPT.to_vec()),
        (
            vec![0; crate::diagnostic_builder::MAX_POLICY_BYTES + 1],
            RECEIPT.to_vec(),
        ),
        (POLICY.to_vec(), vec![]),
        (POLICY.to_vec(), receipt),
        (
            POLICY.to_vec(),
            vec![0; usize::try_from(crate::diagnostic_artifacts::MAX_RECEIPT_BYTES + 1).unwrap()],
        ),
        (POLICY.to_vec(), RECEIPT[..RECEIPT.len() - 1].to_vec()),
    ] {
        exercise(
            &policy,
            &receipt,
            |_| (),
            |_, _| unreachable!(),
            false,
            true,
        );
    }
}

#[test]
fn controller_build_seeded_signature_mutations_refuse_even_with_new_signed_hash() {
    let receipt: crate::diagnostic_artifacts::SignedBuild =
        serde_json::from_slice(RECEIPT).unwrap();
    let mut seed = 0x07542074_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = receipt.clone();
        let mut signature = changed.signature_hex.into_bytes();
        let at = (seed % 128) as usize;
        signature[at] = if signature[at] == b'0' { b'1' } else { b'0' };
        changed.signature_hex = String::from_utf8(signature).unwrap();
        let raw = crate::diagnostic_artifacts::receipt_bytes(&changed).unwrap();
        exercise(
            POLICY,
            &raw,
            |request| request.identity.build_provenance_sha256 = crate::sha256_hex(&raw),
            |_, _| unreachable!(),
            false,
            true,
        );
    }
    println!("seed=0x07542074 signature_mutations=32 production_signing=false");
}

#[test]
fn controller_build_retained_and_original_faults_stay_refused_after_restoration() {
    for case in 0..7 {
        exercise(
            POLICY,
            RECEIPT,
            |_| (),
            |guard, model| {
                match case {
                    0 => guard.policy_bytes[10] ^= 1,
                    1 => guard.receipt_bytes[10] ^= 1,
                    2 => {
                        let mut other = guard.original.request.identity.clone();
                        other.lease_id = "0".repeat(64);
                        guard.build = guard.policy.verify_receipt(RECEIPT, &other).unwrap();
                    }
                    3 => guard.original.request.controller.run_id += 1,
                    4 => guard.original.worker.latch_refusal(),
                    5 => guard.original.controller.refused = true,
                    _ => std::fs::write(model.path(), b"drift\n").unwrap(),
                }
                assert!(guard.revalidate().is_err());
                guard.policy_bytes = POLICY.to_vec();
                guard.receipt_bytes = RECEIPT.to_vec();
                assert!(matches!(
                    guard.revalidate(),
                    Err(ControllerBuildError::Input(TrustInputError::Refused))
                ));
            },
            true,
            true,
        );
    }
}

#[test]
fn controller_build_original_refusal_during_retention_cannot_rebind() {
    exercise(
        POLICY,
        RECEIPT,
        |_| (),
        |guard, _| {
            guard.original.latch_refusal();
            assert!(guard.revalidate().is_err());
        },
        true,
        true,
    );
}

#[test]
fn controller_build_constructor_prior_refusal_cross_latches_without_replacement() {
    for controller_refused in [true, false] {
        let model = Model::new();
        let mut request = model.request();
        request.identity.binary_sha256 = BINARY.into();
        request.identity.build_provenance_sha256 = crate::sha256_hex(RECEIPT);
        let bytes = model.sign(request, &model.key);
        let (_root, _authority_root, mut source, mut authority, worker_bytes) =
            controller_worker_fixture();
        let mut packet = kernel_packet_bytes(&bytes);
        let mut worker = source
            .verify_kernel_policy(&worker_bytes, &mut packet, &mut authority)
            .unwrap();
        let mut controller = model.input();
        {
            let mut original = controller.authenticate_controller(&mut worker).unwrap();
            if controller_refused {
                original.controller.refused = true;
            } else {
                original.worker.latch_refusal();
            }
            assert!(matches!(
                original.bind_reviewed_build(POLICY, RECEIPT),
                Err(ControllerBuildError::Input(TrustInputError::Refused))
            ));
            assert!(original.is_refused());
        }
        assert!(controller.is_refused() && worker.is_refused());
        drop(worker);
        assert!(source.is_refused() && authority.is_failed() && packet.is_refused());
    }
}

#[test]
fn controller_build_independent_bindings_keep_reviewed_builder_pins() {
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    exercise(
                        POLICY,
                        RECEIPT,
                        |_| (),
                        |g, _| g.revalidate().unwrap(),
                        true,
                        false,
                    )
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    println!("independent_threads=4 controller_run_provenance=false");
}
