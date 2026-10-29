use super::validation_trace::{self as trace, Stage};
use super::DiagnosticOperation;

#[test]
fn validation_trace_disabled_and_success_do_not_invent_time_refusals() {
    drop(trace::authentication());
    drop(trace::signature());
    trace::time_refusal(DiagnosticOperation::Reserve, 1, 61, 61);
    let (_, report) = trace::capture(|| {
        for stage in [
            Stage::Unassigned,
            Stage::Setup,
            Stage::Fence,
            Stage::Retain,
            Stage::Observe,
            Stage::Publish,
            Stage::Bind,
            Stage::Checks,
        ] {
            trace::stage(stage);
        }
        trace::stage(Stage::Bind);
        let _call = trace::authentication();
        drop(trace::signature());
        drop(trace::worker_policy());
    });
    assert_eq!(report.stages[Stage::Bind as usize].authentication_calls, 1);
    assert_eq!(report.stages[Stage::Bind as usize].signature_calls, 1);
    assert_eq!(report.stages[Stage::Bind as usize].worker_policy_calls, 1);
    assert!(report.first_time.is_none());
}

#[test]
fn validation_trace_first_time_facts_are_not_replaced_or_authority() {
    let (_, report) = trace::capture(|| {
        trace::stage(Stage::Publish);
        trace::time_refusal(DiagnosticOperation::Reserve, 1, 61, 61);
        trace::stage(Stage::Bind);
        trace::time_refusal(DiagnosticOperation::Heartbeat, 100, 160, 99);
    });
    let first = report.first_time.unwrap();
    assert_eq!((first.issued, first.expires, first.now), (1, 61, 61));
    assert_eq!(first.stage, Stage::Publish);
    assert_eq!(first.operation, DiagnosticOperation::Reserve);
    assert_eq!(report.time_refusals, 2);
}

#[test]
fn validation_trace_nested_unwind_and_ambient_scope_restore_original_frame() {
    let (_, outer) = trace::capture(|| {
        assert!(trace::ambient().is_none());
        let _outer = trace::authentication();
        let (_, inner) = trace::capture(|| drop(trace::signature()));
        assert_eq!(inner.stages[0].signature_calls, 1);
        assert_eq!(inner.stages[0].authentication_calls, 0);
        let payload = std::panic::catch_unwind(|| {
            trace::capture(|| {
                let _early_return = trace::authentication();
                trace::time_refusal(DiagnosticOperation::Reserve, 1, 61, 0);
                panic!("owned validation trace canary");
            })
        })
        .unwrap_err();
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"owned validation trace canary")
        );
    });
    assert_eq!(outer.stages[0].authentication_calls, 1);
    assert_eq!(outer.stages[0].signature_calls, 0);
    assert!(outer.first_time.is_none());
    assert!(trace::capture(|| ()).1.first_time.is_none());
    assert!(trace::ambient().is_some());
}

#[test]
fn validation_trace_independent_threads_keep_numeric_reports_separate() {
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let tasks: Vec<_> = (0..4)
        .map(|i| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                trace::capture(|| {
                    let _call = trace::authentication();
                    trace::time_refusal(DiagnosticOperation::Reserve, i + 1, i + 61, 0);
                    barrier.wait();
                })
                .1
            })
        })
        .collect();
    for (i, task) in tasks.into_iter().enumerate() {
        let report = task.join().unwrap();
        assert_eq!(report.first_time.unwrap().issued, i as u64 + 1);
        assert_eq!(report.stages[0].authentication_calls, 1);
        assert_eq!(report.time_refusals, 1);
    }
    assert!(trace::capture(|| ()).1.first_time.is_none());
}

#[test]
fn validation_trace_actual_authentication_early_error_is_counted_without_signature() {
    let key = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
    let peer = super::DiagnosticPeer {
        uid: 1,
        gids: vec![2],
    };
    let policy = super::DiagnosticPolicy {
        repository_id: 1,
        actor_ids: vec![3],
        client_uids: vec![1],
        client_gid: 2,
        verifying_key: key.verifying_key(),
    };
    let (result, report) = trace::capture(|| super::authenticate(b"", &peer, &policy, 1));
    assert!(matches!(result, Err(super::DiagnosticIpcError::Packet)));
    assert_eq!(report.stages[0].authentication_calls, 1);
    assert_eq!(report.stages[0].signature_calls, 0);
    assert!(report.first_time.is_none());
}

#[test]
fn validation_trace_actual_signed_window_and_signature_errors_are_distinct() {
    use ed25519_dalek::Signer;
    let key = ed25519_dalek::SigningKey::from_bytes(&[8; 32]);
    let peer = super::DiagnosticPeer {
        uid: 1,
        gids: vec![2],
    };
    let policy = super::DiagnosticPolicy {
        repository_id: 1,
        actor_ids: vec![3],
        client_uids: vec![1],
        client_gid: 2,
        verifying_key: key.verifying_key(),
    };
    let request: super::DiagnosticRequest = serde_json::from_value(serde_json::json!({
        "schema_version":1,"request_id":"a".repeat(64),"nonce_sha256":"b".repeat(64),
        "operation":"reserve","expected_state_revision":0,
        "identity":{"lease_id":"c".repeat(64),"binary_sha256":"d".repeat(64),"build_provenance_sha256":"e".repeat(64),"boot_id":"00000000-0000-0000-0000-000000000001"},
        "source_commit":crate::diagnostic_lease::SOURCE_COMMIT,"preset":super::PRESET,
        "controller":{"repository_id":1,"run_id":2,"run_attempt":1,"actor_id":3},
        "issued_at_unix_seconds":100,"expires_at_unix_seconds":160
    })).unwrap();
    for (now, bad_signature) in [(99, false), (159, false), (160, false), (99, true)] {
        let signer = if bad_signature {
            ed25519_dalek::SigningKey::from_bytes(&[9; 32])
        } else {
            key.clone()
        };
        let signed = super::SignedDiagnosticRequest {
            request: request.clone(),
            signature_hex: crate::hex(
                &signer
                    .sign(&super::signing_message(&request).unwrap())
                    .to_bytes(),
            ),
        };
        let bytes = crate::canonical_json(&signed).unwrap();
        let (result, report) = trace::capture(|| super::authenticate(&bytes, &peer, &policy, now));
        assert_eq!(report.stages[0].authentication_calls, 1);
        assert_eq!(report.stages[0].signature_calls, 1);
        if bad_signature {
            assert!(matches!(result, Err(super::DiagnosticIpcError::Signature)));
            assert!(report.first_time.is_none());
        } else if now == 159 {
            assert_eq!(result.unwrap(), request);
            assert!(report.first_time.is_none());
        } else {
            assert!(matches!(result, Err(super::DiagnosticIpcError::Time)));
            let first = report.first_time.unwrap();
            assert_eq!((first.issued, first.expires, first.now), (100, 160, now));
            assert_eq!(first.operation, super::DiagnosticOperation::Reserve);
        }
    }
}
