use hydracache_cluster_testkit::value_plane_cluster_chaos_075::*;
use hydracache_cluster_testkit::value_plane_history_075::{
    HistoryOperation, HistoryResult, ValuePlaneHistoryOracle,
};

fn completed(id: u64, operation: HistoryOperation) -> Vec<ClusterScheduleStep> {
    let read = matches!(operation, HistoryOperation::Read);
    let mut steps = vec![
        ClusterScheduleStep::Invoke {
            id,
            key: b"key".to_vec(),
            operation,
        },
        ClusterScheduleStep::Advance {
            id,
            phase: ClusterPhase::Apply,
        },
    ];
    if !read {
        steps.extend([
            ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Replicate,
            },
            ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Acknowledge,
            },
        ]);
    }
    steps.push(ClusterScheduleStep::Advance {
        id,
        phase: ClusterPhase::Respond,
    });
    steps
}

#[test]
fn staged_actual_owner_decisions_survive_ack_loss_promotion_and_delayed_response() {
    let mut steps = completed(1, HistoryOperation::GetAndPut(b"value".to_vec()));
    steps.pop();
    steps.extend([
        ClusterScheduleStep::Promote { partition: 0 },
        ClusterScheduleStep::Advance {
            id: 1,
            phase: ClusterPhase::Respond,
        },
    ]);
    steps.extend(completed(2, HistoryOperation::Read));
    let report = replay_cluster_schedule(&steps, Default::default()).unwrap();
    assert!(report.linearizable);
    assert_eq!(report.history.calls().len(), 2);
    assert!(report.replica_checks > 0);
    assert!(report.receipt_checks > 0);
    let mut recovery = completed(1, HistoryOperation::PutIfAbsent(b"winner".to_vec()));
    recovery.insert(
        3,
        ClusterScheduleStep::Fault {
            id: 1,
            channel: ClusterChannel::Ack,
            fault: ClusterFault::Drop,
        },
    );
    recovery.insert(
        5,
        ClusterScheduleStep::Fault {
            id: 1,
            channel: ClusterChannel::Ack,
            fault: ClusterFault::Deliver,
        },
    );
    recovery.insert(
        6,
        ClusterScheduleStep::Advance {
            id: 1,
            phase: ClusterPhase::Acknowledge,
        },
    );
    recovery.extend(completed(2, HistoryOperation::Read));
    assert!(
        replay_cluster_schedule(&recovery, Default::default())
            .unwrap()
            .linearizable
    );
}

#[test]
fn seeded_actual_cluster_chaos_checks_histories_bulk_ttl_repair_and_replays_exactly() {
    for seed in [0, 1, 117, 118, u64::MAX] {
        let steps = generated_cluster_schedule(seed, 3).unwrap();
        let a = replay_cluster_schedule(&steps, Default::default()).unwrap();
        let b = replay_cluster_schedule(&steps, Default::default()).unwrap();
        assert_eq!(a.fingerprint, b.fingerprint);
        assert_eq!(a.observations, b.observations);
        assert!(a.linearizable);
        assert_eq!(a.bulk_checks, 3);
        assert_eq!(a.expiry_checks, 3);
    }
}

#[test]
fn independent_history_canary_is_detected_and_shrinks_to_replayable_actual_calls() {
    let steps = generated_cluster_schedule(117, 2).unwrap();
    let broken = |candidate: &[ClusterScheduleStep]| {
        let Ok(report) = replay_cluster_schedule(candidate, Default::default()) else {
            return false;
        };
        let mut history = report.history;
        let mut calls = history.calls().to_vec();
        let Some(call) = calls
            .iter_mut()
            .rev()
            .find(|call| matches!(call.result, HistoryResult::Value(Some(_))))
        else {
            return false;
        };
        call.result = HistoryResult::Value(None);
        history = hydracache_cluster_testkit::value_plane_history_075::ValuePlaneHistory::new(32)
            .unwrap();
        for call in calls {
            history.push(call).unwrap();
        }
        !ValuePlaneHistoryOracle::new(50_000)
            .check(&history)
            .is_linearizable()
    };
    assert!(broken(&steps));
    let shrunk = shrink_cluster_schedule(&steps, 128, broken).unwrap();
    assert!(shrunk.steps.len() < steps.len());
    assert!(broken(&shrunk.steps));
    assert!(replay_cluster_schedule(&shrunk.steps, Default::default()).is_ok());
}

#[test]
fn schedules_enforce_phase_history_input_and_search_bounds_before_claiming_success() {
    let invoke = ClusterScheduleStep::Invoke {
        id: 1,
        key: b"key".to_vec(),
        operation: HistoryOperation::Read,
    };
    assert!(
        replay_cluster_schedule(&[invoke.clone(), invoke.clone()], Default::default()).is_err()
    );
    assert!(replay_cluster_schedule(
        &[
            invoke.clone(),
            ClusterScheduleStep::Advance {
                id: 1,
                phase: ClusterPhase::Respond
            }
        ],
        Default::default()
    )
    .is_err());
    assert!(replay_cluster_schedule(
        &[invoke],
        ClusterScheduleBounds {
            max_pending: 0,
            ..Default::default()
        }
    )
    .is_err());
    assert!(generated_cluster_schedule(1, 0).is_err());
    assert!(generated_cluster_schedule(1, 100).is_err());
    assert!(shrink_cluster_schedule(&[], 0, |_| false).is_err());
    let oversized = completed(1, HistoryOperation::GetAndPut(vec![0; 4097]));
    assert!(replay_cluster_schedule(&oversized, Default::default()).is_err());
    assert!(encode_cluster_schedule(&oversized).is_err());
    let mut steps = completed(1, HistoryOperation::GetAndPut(b"v".to_vec()));
    steps.extend(completed(2, HistoryOperation::Read));
    assert!(matches!(
        replay_cluster_schedule(
            &steps,
            ClusterScheduleBounds {
                max_search_states: 1,
                ..Default::default()
            }
        ),
        Err(ClusterScheduleError::SearchBudget)
    ));
}

#[test]
fn hostile_fixture_decoder_rejects_future_unknown_duplicate_and_oversized_shapes() {
    let steps = generated_cluster_schedule(117, 1).unwrap();
    let encoded = encode_cluster_schedule(&steps).unwrap();
    assert_eq!(decode_cluster_schedule(&encoded).unwrap(), steps);
    for input in [b"{}".as_slice(),b"{\"version\":2,\"steps\":[]}",b"{\"version\":1,\"steps\":[],\"extra\":1}",b"{\"version\":1,\"steps\":[{\"kind\":\"invoke\",\"id\":1,\"operation\":\"read\",\"key\":\"ff0\"}]}",b"{\"version\":1,\"steps\":[{\"kind\":\"bogus\"}]}"] {
        assert!(decode_cluster_schedule(input).is_err());
    }
    assert!(decode_cluster_schedule(&vec![b' '; 65537]).is_err());
    for byte in 0..=255 {
        let input = vec![byte; 64];
        let _ = fuzz_cluster_schedule(&input);
    }
}

#[test]
fn impossible_history_at_exact_search_limit_is_a_violation_not_exhaustion() {
    use hydracache_cluster_testkit::value_plane_history_075::{HistoryCall, ValuePlaneHistory};
    use hydracache_cluster_testkit::value_plane_model_075::{CanonicalMapKey, OutcomeCertainty};
    let mut history = ValuePlaneHistory::new(1).unwrap();
    history
        .push(HistoryCall::completed(
            1,
            CanonicalMapKey::new("t", "n", 1, b"k".to_vec(), 0),
            HistoryOperation::Read,
            0,
            1,
            HistoryResult::Value(Some(b"impossible".to_vec())),
            OutcomeCertainty::Certain,
        ))
        .unwrap();
    assert!(matches!(
        verify_cluster_history(&history, 1),
        Err(ClusterScheduleError::Violation(_))
    ));
    assert!(verify_cluster_history(&history, 0).is_err());
}

#[test]
fn conditional_returns_and_binary_fault_retries_use_one_real_decision() {
    let mut steps = Vec::new();
    let operations = [
        HistoryOperation::Put(b"a".to_vec()),
        HistoryOperation::ReplaceIfPresent(b"b".to_vec()),
        HistoryOperation::ReplaceIfValue {
            expected: b"wrong".to_vec(),
            value: b"bad".to_vec(),
        },
        HistoryOperation::ReplaceIfValue {
            expected: b"b".to_vec(),
            value: b"c".to_vec(),
        },
        HistoryOperation::RemoveIfValue {
            expected: b"wrong".to_vec(),
        },
        HistoryOperation::RemoveIfValue {
            expected: b"c".to_vec(),
        },
        HistoryOperation::Delete,
        HistoryOperation::GetAndPut(b"final".to_vec()),
        HistoryOperation::Read,
    ];
    for (index, operation) in operations.into_iter().enumerate() {
        steps.extend(completed(index as u64 + 1, operation));
    }
    let bytes = encode_cluster_schedule(&steps).unwrap();
    assert_eq!(decode_cluster_schedule(&bytes).unwrap(), steps);
    assert!(fuzz_cluster_schedule(&bytes).unwrap().linearizable);
    for channel in [
        ClusterChannel::Request,
        ClusterChannel::Replica,
        ClusterChannel::Ack,
        ClusterChannel::Response,
    ] {
        for fault in [
            ClusterFault::Drop,
            ClusterFault::Duplicate,
            ClusterFault::Delay,
            ClusterFault::Corrupt,
            ClusterFault::Partial,
        ] {
            let id = 1;
            let mut candidate = vec![ClusterScheduleStep::Invoke {
                id,
                key: b"key".to_vec(),
                operation: HistoryOperation::GetAndPut(b"v".to_vec()),
            }];
            // Request faults are exercised before owner application. Other faults are
            // held until the final proof attempt so an explicit retry recovers the receipt.
            if channel == ClusterChannel::Request {
                candidate.extend([
                    ClusterScheduleStep::Fault { id, channel, fault },
                    ClusterScheduleStep::Advance {
                        id,
                        phase: ClusterPhase::Apply,
                    },
                    ClusterScheduleStep::Fault {
                        id,
                        channel,
                        fault: ClusterFault::Deliver,
                    },
                ]);
            }
            let already_applied = channel == ClusterChannel::Request
                && matches!(fault, ClusterFault::Duplicate | ClusterFault::Delay);
            if !already_applied {
                candidate.push(ClusterScheduleStep::Advance {
                    id,
                    phase: ClusterPhase::Apply,
                });
            }
            candidate.push(ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Replicate,
            });
            if channel != ClusterChannel::Request {
                candidate.push(ClusterScheduleStep::Fault { id, channel, fault });
            }
            candidate.push(ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Acknowledge,
            });
            if channel != ClusterChannel::Request
                && !matches!(fault, ClusterFault::Duplicate | ClusterFault::Delay)
            {
                candidate.extend([
                    ClusterScheduleStep::Fault {
                        id,
                        channel,
                        fault: ClusterFault::Deliver,
                    },
                    ClusterScheduleStep::Advance {
                        id,
                        phase: ClusterPhase::Acknowledge,
                    },
                ]);
            }
            candidate.push(ClusterScheduleStep::Advance {
                id,
                phase: ClusterPhase::Respond,
            });
            let encoded = encode_cluster_schedule(&candidate).unwrap();
            assert_eq!(decode_cluster_schedule(&encoded).unwrap(), candidate);
            assert!(
                replay_cluster_schedule(&candidate, Default::default()).is_ok(),
                "{channel:?}/{fault:?}"
            );
        }
    }
}

#[test]
fn unreachable_owner_and_invalid_controls_never_invent_a_successful_absence() {
    use hydracache_cluster_testkit::value_plane_service_cluster_075::{
        ServiceClusterBounds, ServiceClusterHarness,
    };
    let cluster = ServiceClusterHarness::new(
        &["a", "b", "c"],
        ServiceClusterBounds::default(),
        Default::default(),
        Default::default(),
        1,
    )
    .unwrap();
    let key = (0..=255)
        .map(|n| vec![n])
        .find(|key| cluster.partition_for(key) == 0)
        .unwrap();
    let mut steps = vec![
        ClusterScheduleStep::Invoke {
            id: 1,
            key: key.clone(),
            operation: HistoryOperation::GetAndPut(b"v".to_vec()),
        },
        ClusterScheduleStep::SetLive {
            node: 0,
            live: false,
        },
        ClusterScheduleStep::Advance {
            id: 1,
            phase: ClusterPhase::Apply,
        },
        ClusterScheduleStep::Promote { partition: 0 },
        ClusterScheduleStep::Repair { partition: 0 },
    ];
    for phase in [
        ClusterPhase::Apply,
        ClusterPhase::Replicate,
        ClusterPhase::Acknowledge,
        ClusterPhase::Respond,
    ] {
        steps.push(ClusterScheduleStep::Advance { id: 1, phase });
    }
    steps.extend([
        ClusterScheduleStep::SetLive {
            node: 0,
            live: true,
        },
        ClusterScheduleStep::Invoke {
            id: 2,
            key,
            operation: HistoryOperation::Read,
        },
        ClusterScheduleStep::Advance {
            id: 2,
            phase: ClusterPhase::Apply,
        },
        ClusterScheduleStep::Advance {
            id: 2,
            phase: ClusterPhase::Respond,
        },
    ]);
    let encoded = encode_cluster_schedule(&steps).unwrap();
    assert_eq!(decode_cluster_schedule(&encoded).unwrap(), steps);
    let report = replay_cluster_schedule(&steps, Default::default()).unwrap();
    assert!(report
        .observations
        .iter()
        .any(|seen| seen.contains("blocked:NodeUnavailable")));
    for invalid in [
        ClusterScheduleStep::SetLive {
            node: 3,
            live: true,
        },
        ClusterScheduleStep::Repair { partition: 3 },
    ] {
        assert!(replay_cluster_schedule(&[invalid], Default::default()).is_err());
    }
}
