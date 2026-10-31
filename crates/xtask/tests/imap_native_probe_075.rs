use serde_json::{json, Value};
use xtask::imap_native_probe_075::*;
fn probe() -> Value {
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
        "artifact_sha256":xtask::imap_pre074_075::HAZELCAST_SHA256,"point_calls":3,"second_put_kinds":["replaced","inserted"],
        "second_put_previous":["0080ff",null],"size":2,"is_empty":false,"contains_entry":true,"fallback":"0080ff","scan_count":2,"native_version":1,
        "native_ttl_millis":0,"native_event_count":3,"snapshot_digest":xtask::imap_qualification_075::digest(&bytes),"removed_previous":["61",null],
        "put_all_origin":"native_aggregate","clear_event":"map_cleared","clear_affected":1,"final_digest":xtask::imap_qualification_075::digest(&[])})
}
#[test]
fn native_fields_are_closed_and_each_required_observation_has_an_independent_expectation() {
    let value = probe();
    validate(&serde_json::from_value(value.clone()).unwrap()).unwrap();
    let mut changed = value.clone();
    changed["native_ttl_millis"] = json!(i64::MAX);
    validate(&serde_json::from_value(changed).unwrap()).unwrap();
    for key in [
        "schema",
        "profile",
        "artifact_sha256",
        "fallback",
        "snapshot_digest",
        "put_all_origin",
        "clear_event",
        "final_digest",
    ] {
        let mut changed = value.clone();
        changed[key] = json!("foreign");
        assert!(
            validate(&serde_json::from_value(changed).unwrap()).is_err(),
            "{key}"
        );
    }
    for key in [
        "point_calls",
        "size",
        "scan_count",
        "native_ttl_millis",
        "native_event_count",
        "clear_affected",
    ] {
        let mut changed = value.clone();
        changed[key] = json!(9);
        assert!(
            validate(&serde_json::from_value(changed).unwrap()).is_err(),
            "{key}"
        );
    }
    for key in ["is_empty", "contains_entry"] {
        let mut changed = value.clone();
        changed[key] = json!(!value[key].as_bool().unwrap());
        assert!(
            validate(&serde_json::from_value(changed).unwrap()).is_err(),
            "{key}"
        );
    }
    for key in [
        "observed_fields",
        "second_put_kinds",
        "second_put_previous",
        "removed_previous",
    ] {
        let mut changed = value.clone();
        changed[key] = json!([]);
        assert!(
            validate(&serde_json::from_value(changed).unwrap()).is_err(),
            "{key}"
        );
    }
    for key in ["owners", "tokens", "watermarks", "logical_tick"] {
        let mut changed = value.clone();
        changed[key] = json!(0);
        assert!(
            serde_json::from_value::<NativeProbe>(changed).is_err(),
            "{key}"
        );
    }
    let mut changed = value;
    changed.as_object_mut().unwrap().remove("observed_fields");
    assert!(serde_json::from_value::<NativeProbe>(changed).is_err());
}
#[test]
fn native_cli_and_missing_or_substituted_artifacts_refuse_before_java_launch() {
    assert!(run(vec![]).is_err());
    assert!(run(vec!["--release".into(), "0.74".into()]).is_err());
    let root = tempfile::tempdir().unwrap();
    assert!(observe(root.path()).is_err());
    for path in [
        "tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar",
        "tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar",
        "tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.jar",
    ] {
        let path = root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"foreign").unwrap();
    }
    assert!(observe(root.path())
        .unwrap_err()
        .to_string()
        .contains("pin differs"));
}
