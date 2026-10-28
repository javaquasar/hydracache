use super::*;
use serde_json::Value;

#[test]
fn original_start_consent_id_is_new_bounded_and_canonical() {
    let reservation = "a".repeat(64);
    let response = "b".repeat(64);
    require_new_id(&"c".repeat(64), &reservation, &response).unwrap();
    for id in [
        reservation.clone(),
        response.clone(),
        String::new(),
        "c".repeat(63),
        "c".repeat(65),
        "C".repeat(64),
        "g".repeat(64),
    ] {
        assert!(matches!(
            require_new_id(&id, &reservation, &response),
            Err(OriginalStartConsentError::Binding)
        ));
    }
}

// Every leaf comparison is exact, even where a mutation is not valid codec grammar.
fn leaves(v: &Value, path: String, out: &mut Vec<String>) {
    if let Value::Object(map) = v {
        for (key, value) in map {
            leaves(value, format!("{path}/{key}"), out);
        }
    } else {
        out.push(path);
    }
}
#[test]
fn original_start_consent_frozen_body_all_leaves_are_exact() {
    // Synthetic untrusted data, never a runtime owner or caller admission clock.
    let expected: StartConsentBody=serde_json::from_value(serde_json::json!({
        "schema_version":BODY_SCHEMA,"action":"start-cell-one",
        "consent_request_id":"1".repeat(64),"reservation_request_id":"2".repeat(64),
        "controller_attempt_sha256":"3".repeat(64),"collector_reserved_sha256":"4".repeat(64),
        "reserved_state_sha256":"5".repeat(64),"challenge_nonce_sha256":"6".repeat(64),
        "authenticated_response_sha256":"7".repeat(64),"worker_policy_sha256":"8".repeat(64),
        "worker_policy_epoch":1,"collector_unit_sha256":"9".repeat(64),
        "controller":{"repository_id":1,"run_id":2,"run_attempt":3,"actor_id":4},
        "source_commit":crate::diagnostic_lease::SOURCE_COMMIT,"preset":crate::diagnostic_ipc::PRESET,
        "starting_state":{
            "identity":{"lease_id":"a".repeat(64),"boot_id":"boot","binary_sha256":"b".repeat(64),"build_provenance_sha256":"c".repeat(64)},
            "revision":2,"stage":"starting","completed_cells":0,
            "reserved_monotonic_ns":1,"controller_monotonic_ns":2,"last_observed_monotonic_ns":2,
            "cell_started_monotonic_ns":2,"cgroup_inode":null,"reason":null,
            "cleanup_confirmed":false,"promotable":false,"admission_allowed":false},
        "issued_at_unix_seconds":100,"expires_at_unix_seconds":160
    })).unwrap();
    equal_body(&expected, &expected).unwrap();
    let value = serde_json::to_value(&expected).unwrap();
    let mut paths = Vec::new();
    leaves(&value, String::new(), &mut paths);
    assert_eq!(paths.len(), 36);
    let mut seed = 0x0740c0a3_u64;
    for i in (1..paths.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        paths.swap(i, seed as usize % (i + 1));
    }
    for path in paths {
        let mut changed = value.clone();
        let v = changed.pointer_mut(&path).unwrap();
        *v = match v {
            Value::String(s) => Value::String(if s == "starting" {
                "reserved".into()
            } else {
                format!("{s}a")
            }),
            Value::Number(n) => serde_json::json!(n.as_u64().unwrap() + 1),
            Value::Bool(b) => Value::Bool(!*b),
            Value::Null => {
                if path.ends_with("reason") {
                    Value::String("controller_lost".into())
                } else {
                    serde_json::json!(1)
                }
            }
            _ => unreachable!(),
        };
        let actual: StartConsentBody = serde_json::from_value(changed).unwrap();
        assert!(
            matches!(
                equal_body(&expected, &actual),
                Err(OriginalStartConsentError::Binding)
            ),
            "{path}"
        );
    }
    println!("original_start_consent seed=0x0740c0a3 exact_binding_mutations=36");
}
