use serde_json::{json, Value};
use xtask::imap_native_recovery_075::*;
fn fixture() -> Value {
    json!({"schema":"imap075-native-recovery-v1","profile":"controlled_unique_point_writers_queue_overflow","artifact_sha256":xtask::imap_pre074_075::HAZELCAST_SHA256,
    "accepted_writes":8,"callbacks_observed":8,"correlated_writes":8,"queue_capacity":2,"gap_before":"queue_overflow","old_generation":1,"new_generation":2,
    "baseline_key":"61","baseline_value":"07","baseline_digest":xtask::imap_qualification_075::digest(&[0,0,0,1,97,0,0,0,1,7,0]),
    "post_event_generation":2,"post_event_kind":"updated","post_event_key":"61","post_event_old_value":"07","post_event_value":"08","state_after_recovery":"healthy","state_after_close":"closed",
    "removed_subscriptions":2,"client_alive_after_close":true,"map_value_after_close":"08"})
}
#[test]
fn every_fixed_native_recovery_observation_is_required_and_independently_checked() {
    let value = fixture();
    validate(&serde_json::from_value(value.clone()).unwrap()).unwrap();
    for (key, original) in value.as_object().unwrap() {
        let mut changed = value.clone();
        changed[key] = match original {
            Value::String(_) => json!("foreign"),
            Value::Number(_) => json!(99),
            Value::Bool(v) => json!(!v),
            _ => panic!(),
        };
        assert!(
            validate(&serde_json::from_value(changed).unwrap()).is_err(),
            "{key}"
        );
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<RecoveryProbe>(missing).is_err(),
            "{key}"
        );
    }
    let mut future = value;
    future["server_watermark"] = json!(2);
    assert!(serde_json::from_value::<RecoveryProbe>(future).is_err());
}
#[test]
fn schema_and_cli_enforce_local_scope_and_fixed_artifact_paths() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../docs/testing/imap/0.75/schemas/native-recovery.schema.json"
    ))
    .unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&fixture())
        .unwrap();
    assert!(run(vec![]).is_err());
    assert!(run(vec!["--release".into(), "0.74".into()]).is_err());
    assert!(observe(tempfile::tempdir().unwrap().path()).is_err());
}
