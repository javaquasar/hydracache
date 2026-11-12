use super::*;
use serde_json::{json, Value};
fn native() -> Value {
    let mut bytes = Vec::new();
    for (key, value) in [
        (vec![0u8, 255], vec![97u8]),
        (vec![97u8], vec![0u8, 128, 255]),
    ] {
        for value in [key, value] {
            bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
            bytes.extend(value);
        }
        bytes.push(0);
    }
    json!({"schema":"imap075-native-observation-v1","profile":"native-exclusive-eternal-v1","observed_fields":["raw_return","final_digest","native_events"],
        "artifact_sha256":crate::imap_pre074_075::HAZELCAST_SHA256,"point_calls":3,"second_put_kinds":["replaced","inserted"],
        "second_put_previous":["0080ff",null],"size":2,"is_empty":false,"contains_entry":true,"fallback":"0080ff","scan_count":2,"native_version":1,
        "native_ttl_millis":0,"native_event_count":3,"snapshot_digest":crate::imap_qualification_075::digest(&bytes),"removed_previous":["61",null],
        "put_all_origin":"native_aggregate","clear_event":"map_cleared","clear_affected":1,"final_digest":crate::imap_qualification_075::digest(&[])})
}

fn recovery() -> Value {
    json!({"schema":"imap075-native-recovery-v1","profile":"controlled_unique_point_writers_queue_overflow","artifact_sha256":crate::imap_pre074_075::HAZELCAST_SHA256,
    "accepted_writes":8,"callbacks_observed":8,"correlated_writes":8,"queue_capacity":2,"gap_before":"queue_overflow","old_generation":1,"new_generation":2,
    "baseline_key":"61","baseline_value":"07","baseline_digest":crate::imap_qualification_075::digest(&[0,0,0,1,97,0,0,0,1,7,0]),
    "post_event_generation":2,"post_event_kind":"updated","post_event_key":"61","post_event_old_value":"07","post_event_value":"08","state_after_recovery":"healthy","state_after_close":"closed",
    "removed_subscriptions":2,"client_alive_after_close":true,"map_value_after_close":"08"})
}
#[test]
fn preparation_smoke_requires_exact_owned_paths_and_complete_200ms_segments() {
    use crate::imap_soak_075::Terminal;
    let directory = Path::new("target/imap-075/unit-preparation");
    for cohort in ["reference", "native", "daemon"] {
        let mut reference = Artifact {
            path: format!("target/imap-075/unit-preparation/soak-{cohort}/report.json"),
            sha256: "0".repeat(64),
            bytes: 1,
        };
        validate_soak_report_path(directory, &reference, cohort).unwrap();
        for path in [
            "foreign/report.json",
            "target/imap-075/unit-preparation/soak-reference/../report.json",
        ] {
            reference.path = path.into();
            assert!(validate_soak_report_path(directory, &reference, cohort).is_err());
        }
    }
    validate_preparation_soak(Terminal::Complete, 200).unwrap();
    for terminal in [Terminal::Cancelled, Terminal::TimedOut, Terminal::Failed] {
        assert!(validate_preparation_soak(terminal, 200).is_err());
    }
    for duration in [0, 199, 201, u64::MAX] {
        assert!(validate_preparation_soak(Terminal::Complete, duration).is_err());
    }
}
#[test]
fn typed_extended_replay_requires_identical_observations_even_for_individually_valid_ttl_samples() {
    let saved: crate::imap_native_probe_075::NativeProbe =
        serde_json::from_value(native()).unwrap();
    let mut fresh = saved.clone();
    validate_extended_replay(&saved, &fresh).unwrap();
    fresh.native_ttl_millis = i64::MAX as u64;
    crate::imap_native_probe_075::validate(&saved).unwrap();
    crate::imap_native_probe_075::validate(&fresh).unwrap();
    assert!(validate_extended_replay(&saved, &fresh)
        .unwrap_err()
        .to_string()
        .contains("native extended replay differs"));
}
#[test]
fn typed_listener_replay_refuses_any_substituted_recovery_observation() {
    let saved: crate::imap_native_recovery_075::RecoveryProbe =
        serde_json::from_value(recovery()).unwrap();
    crate::imap_native_recovery_075::validate(&saved).unwrap();
    validate_recovery_replay(&saved, &saved).unwrap();
    let mut changed = saved.clone();
    changed.map_value_after_close = "ff".into();
    assert!(validate_recovery_replay(&saved, &changed)
        .unwrap_err()
        .to_string()
        .contains("native listener recovery replay differs"));
}
