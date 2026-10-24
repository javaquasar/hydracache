use super::*;
use hydracache_long_run_supervisor_074::diagnostic_worker_policy::intent_projection::collector::*;

fn body(f: &Fixture) -> IntentProjectionBody {
    IntentProjectionBody {
        schema_version: COLLECTOR_INTENT_SCHEMA.into(),
        unit_spec_sha256: project_collector_reserved_state(&f.state)
            .unwrap()
            .unit_spec_sha256,
        ..f.body()
    }
}
fn signed(f: &Fixture, body: &IntentProjectionBody) -> Vec<u8> {
    line(&SignedCollectorIntentProjection {
        schema_version: SIGNED_COLLECTOR_INTENT_SCHEMA.into(),
        intent: body.clone(),
        signature_hex: hex(&f
            .key
            .sign(&collector_intent_signing_message(body).unwrap())
            .to_bytes()),
    })
}
fn reject(f: &Fixture, bytes: &[u8], now: u64) -> IntentProjectionError {
    let mut worker = f.worker();
    let error = verify_collector_intent_projection(&mut worker, bytes, &f.check(now))
        .err()
        .unwrap();
    assert!(
        !worker.is_refused(),
        "packet error must not revoke healthy worker"
    );
    error
}

#[test]
fn collector_four_modeled_cells_keep_distinct_exact_pins() {
    let mut hashes = std::collections::BTreeSet::new();
    for cell in 0..4 {
        let f = Fixture::new(cell);
        let raw = signed(&f, &body(&f));
        let mut worker = f.worker();
        let mut guard =
            verify_collector_intent_projection(&mut worker, &raw, &f.check(100)).unwrap();
        assert_eq!(guard.intent_sha256(), digest(&raw));
        for now in [100, 101, 159] {
            guard.revalidate(&raw, &f.check(now)).unwrap();
        }
        assert!(!guard.is_refused());
        assert!(hashes.insert(
            project_collector_reserved_state(&f.state)
                .unwrap()
                .unit_spec_sha256
        ));
    }
}

#[test]
fn collector_projection_removes_regular_file_access_and_preserves_other_properties() {
    for cell in 0..4 {
        let f = Fixture::new(cell);
        let raw = collector_unit_projection(&f.state).unwrap();
        let v: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(line(&v), raw);
        assert_eq!(v["schema_version"], COLLECTOR_UNIT_SCHEMA);
        let old = hydracache_long_run_supervisor_074::diagnostic_unit::build_diagnostic_unit_spec(
            &f.state,
        )
        .unwrap();
        let p = v["worker"]["properties"].as_array().unwrap();
        assert_eq!(v["worker"]["unit_name"], old.unit_name);
        assert_eq!(p.len(), old.properties.len() - 2);
        let names: Vec<_> = p.iter().map(|p| p[0].as_str().unwrap()).collect();
        assert!(!names
            .iter()
            .any(|n| n.contains("OutputFile") || n.contains("ErrorFile")));
        let actual: Vec<_> = p
            .iter()
            .filter(|p| {
                ![
                    "Description",
                    "WorkingDirectory",
                    "ReadWritePaths",
                    "LimitFSIZE",
                ]
                .contains(&p[0].as_str().unwrap())
            })
            .cloned()
            .collect();
        // Independent old typed projection: property and command ordering must survive.
        let old_raw: Value = serde_json::to_value(old.properties.iter().map(|(name, value)| {
            use hydracache_long_run_supervisor_074::systemd_unit::UnitProperty::*;
            let (tag, val) = match value {
                Text(v) => ("text", json!(v)), Boolean(v) => ("boolean", json!(v)),
                Unsigned(v) => ("unsigned", json!(v)), Bytes(v) => ("bytes", json!(v)),
                Strings(v) => ("strings", json!(v)), Commands(v) => ("commands", json!(v.iter().map(|c| json!({"path":c.path,"argv":c.argv,"ignore_failure":c.ignore_failure})).collect::<Vec<_>>())),
            }; json!([name, {"type":tag,"value":val}])
        }).collect::<Vec<_>>()).unwrap();
        let expected: Vec<_> = old_raw
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| {
                ![
                    "Description",
                    "WorkingDirectory",
                    "ReadWritePaths",
                    "LimitFSIZE",
                    "StandardOutputFileToAppend",
                    "StandardErrorFileToAppend",
                ]
                .contains(&p[0].as_str().unwrap())
            })
            .cloned()
            .collect();
        assert_eq!(actual, expected);
        for (name, tag, val) in [
            ("WorkingDirectory", "text", json!("/")),
            ("ReadWritePaths", "strings", json!([])),
            ("LimitFSIZE", "unsigned", json!(0)),
        ] {
            assert_eq!(
                p.iter().find(|p| p[0] == name).unwrap(),
                &json!([name,{"type":tag,"value":val}])
            );
        }
        assert_eq!(
            v["output"],
            json!({
                "transport":"original-owned-pipes", "slots":[["stdout","collector-stdout.bin"],["stderr","collector-stderr.bin"]],
                "sink_namespace":"collector-attempt-v1", "sink_owner_uid":0, "sink_directory_mode":"0700",
                "sink_creation":"exclusive-no-follow-no-adoption", "evidence_writer":"sole-root-collector",
                "stream_byte_limit":8_388_608, "combined_byte_limit":16_777_216,
                "completion_requires":["authenticated-original-start","original-cgroup-revoked-and-empty",
                    "all-creator-manager-writer-copies-closed","both-kernel-read-eofs-after-drain",
                    "exact-byte-and-hash-reconciliation","files-synced-and-readonly","directory-synced",
                    "all-evidence-writers-explicitly-closed"]
            })
        );
    }
}

#[test]
fn collector_and_legacy_domains_and_output_digests_cannot_be_relabelled() {
    let f = Fixture::new(0);
    let new = body(&f);
    assert_eq!(
        reject(&f, &f.signed(&f.body()), 100),
        IntentProjectionError::Document
    );
    assert_eq!(
        f.reject(&signed(&f, &new), 100),
        IntentProjectionError::Document
    );
    for domain in [
        INTENT_DOMAIN,
        WORKER_POLICY_DOMAIN,
        b"hydracache-diagnostic-request-074-v1",
    ] {
        let mut message = domain.to_vec();
        message.push(0);
        message.extend(line(&new));
        let bad = line(&SignedCollectorIntentProjection {
            schema_version: SIGNED_COLLECTOR_INTENT_SCHEMA.into(),
            intent: new.clone(),
            signature_hex: hex(&f.key.sign(&message).to_bytes()),
        });
        assert_eq!(reject(&f, &bad, 100), IntentProjectionError::Signature);
    }
    let old = f.body();
    let mut message = COLLECTOR_INTENT_DOMAIN.to_vec();
    message.push(0);
    message.extend(line(&old));
    let bad = line(&SignedIntentProjection {
        schema_version: SIGNED_INTENT_SCHEMA.into(),
        intent: old.clone(),
        signature_hex: hex(&f.key.sign(&message).to_bytes()),
    });
    assert_eq!(f.reject(&bad, 100), IntentProjectionError::Signature);
    assert_eq!(
        reject(
            &f,
            &signed(
                &f,
                &IntentProjectionBody {
                    unit_spec_sha256: old.unit_spec_sha256,
                    ..new.clone()
                }
            ),
            100
        ),
        IntentProjectionError::Binding
    );
    assert_eq!(
        f.reject(
            &f.signed(&IntentProjectionBody {
                unit_spec_sha256: new.unit_spec_sha256,
                ..old
            }),
            100
        ),
        IntentProjectionError::Binding
    );
    assert_eq!(
        collector_intent_signing_message(&f.body()),
        Err(IntentProjectionError::Document)
    );
}

#[test]
fn collector_document_budget_canonical_schema_and_duplicate_fields_refuse() {
    let f = Fixture::new(0);
    let good = signed(&f, &body(&f));
    let original: Value = serde_json::from_slice(&good).unwrap();
    let mut bads = vec![
        Vec::new(),
        vec![b' '; MAX_SIGNED_INTENT_BYTES + 1],
        good[..good.len() - 1].to_vec(),
        serde_json::to_vec_pretty(&original).unwrap(),
        format!(
            "{{\"schema_version\":\"x\",{}",
            std::str::from_utf8(&good[1..]).unwrap()
        )
        .into_bytes(),
    ];
    for pointer in ["/future", "/intent/future", "/intent/controller/future"] {
        let mut v = original.clone();
        if pointer == "/future" {
            v["future"] = json!(1);
        } else if pointer == "/intent/future" {
            v["intent"]["future"] = json!(1);
        } else {
            v["intent"]["controller"]["future"] = json!(1);
        }
        bads.push(line(&v));
    }
    for (field, value) in [
        ("schema_version", json!(INTENT_SCHEMA)),
        (
            "schema_version",
            json!("diagnostic-collector-intent-074-v2"),
        ),
        ("nonce_sha256", json!("A".repeat(64))),
        ("worker_policy_epoch", json!(0)),
        ("request_id", json!("x".repeat(MAX_INTENT_BODY_BYTES))),
    ] {
        let mut v = original.clone();
        v["intent"][field] = value;
        let mut message = COLLECTOR_INTENT_DOMAIN.to_vec();
        message.push(0);
        message.extend(line(&v["intent"]));
        v["signature_hex"] = json!(hex(&f.key.sign(&message).to_bytes()));
        bads.push(line(&v));
    }
    for schema in [
        SIGNED_INTENT_SCHEMA,
        "diagnostic-signed-collector-intent-074-v2",
    ] {
        let mut v = original.clone();
        v["schema_version"] = json!(schema);
        bads.push(line(&v));
    }
    // Nested duplicate field must refuse before signature/binding checks.
    bads.push(
        String::from_utf8(good.clone())
            .unwrap()
            .replace("\"intent\":{", "\"intent\":{\"request_id\":\"x\",")
            .into_bytes(),
    );
    for bad in bads {
        assert_eq!(reject(&f, &bad, 100), IntentProjectionError::Document);
    }
    let mut huge = body(&f);
    huge.request_id = "x".repeat(MAX_INTENT_BODY_BYTES);
    assert_eq!(
        collector_intent_signing_message(&huge),
        Err(IntentProjectionError::Document)
    );
}

#[test]
fn collector_strict_signature_key_and_encoding() {
    let f = Fixture::new(0);
    let b = body(&f);
    let raw = signed(&f, &b);
    let mut doc: SignedCollectorIntentProjection = serde_json::from_slice(&raw).unwrap();
    for encoding in ["A".repeat(128), "f".repeat(126), "g".repeat(128)] {
        doc.signature_hex = encoding;
        assert_eq!(
            reject(&f, &line(&doc), 100),
            IntentProjectionError::Document
        );
    }
    doc.signature_hex = hex(&SigningKey::from_bytes(&[76; 32])
        .sign(&collector_intent_signing_message(&b).unwrap())
        .to_bytes());
    assert_eq!(
        reject(&f, &line(&doc), 100),
        IntentProjectionError::Signature
    );
}

#[test]
fn collector_every_output_requirement_is_hashed_not_supplied_as_authority() {
    let f = Fixture::new(0);
    let projection: Value =
        serde_json::from_slice(&collector_unit_projection(&f.state).unwrap()).unwrap();
    let initial = body(&f);
    let fields: Vec<_> = projection["output"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    for field in &fields {
        let mut changed = projection.clone();
        changed["output"][field] = Value::Null;
        let altered = digest(&line(&changed));
        assert_ne!(initial.unit_spec_sha256, altered);
        assert_eq!(
            reject(
                &f,
                &signed(
                    &f,
                    &IntentProjectionBody {
                        unit_spec_sha256: altered,
                        ..initial.clone()
                    }
                ),
                100
            ),
            IntentProjectionError::Binding
        );
    }
    for field in ["slots", "completion_requires"] {
        let mut changed = projection.clone();
        changed["output"][field].as_array_mut().unwrap().reverse();
        assert_ne!(initial.unit_spec_sha256, digest(&line(&changed)));
    }
    let bytes = signed(&f, &initial);
    // Byte verification is deliberately not global once-only execution admission.
    for _ in 0..3 {
        let mut worker = f.worker();
        let guard = verify_collector_intent_projection(&mut worker, &bytes, &f.check(100)).unwrap();
        assert!(!guard.is_refused());
    }
    println!(
        "collector_output_requirements fields={} order_mutations=2 durable_replay_prevention=false",
        fields.len()
    );
}

#[test]
fn collector_window_is_bounded_and_expiry_exclusive() {
    let f = Fixture::new(0);
    let b = body(&f);
    let raw = signed(&f, &b);
    for now in [0, 99, 160, u64::MAX] {
        assert_eq!(reject(&f, &raw, now), IntentProjectionError::Time);
    }
    for (issued, expires) in [
        (0, 60),
        (100, 100),
        (101, 100),
        (100, 161),
        (u64::MAX, u64::MAX),
    ] {
        let bad = IntentProjectionBody {
            issued_at_unix_seconds: issued,
            expires_at_unix_seconds: expires,
            ..b.clone()
        };
        assert_eq!(
            reject(&f, &signed(&f, &bad), 100),
            IntentProjectionError::Time
        );
    }
}

#[test]
fn collector_principal_configuration_stays_independent_and_bounded() {
    for mutation in 0..13 {
        let mut f = Fixture::new(0);
        let raw = signed(&f, &body(&f));
        match mutation {
            0 => f.peer.uid = 0,
            1 => f.peer.gids.clear(),
            2 => f.peer.gids = vec![902, 902],
            3 => f.controller.actor_ids = vec![43],
            4 => f.controller.client_uids = vec![901],
            5 => f.controller.actor_ids = vec![42, 42],
            6 => f.controller.client_gid = 0,
            7 => {
                f.controller.verifying_key =
                    ed25519_dalek::VerifyingKey::from_bytes(&[0; 32]).unwrap()
            }
            8 => f.controller.actor_ids = (1..=33).collect(),
            9 => f.controller.client_uids = (900..933).collect(),
            10 => f.peer.gids = (900..933).collect(),
            11 => f.controller.actor_ids.reverse(),
            _ => f.controller.client_uids.clear(),
        }
        assert_eq!(reject(&f, &raw, 100), IntentProjectionError::Principal);
    }
    let f = Fixture::new(0);
    for (field, value) in [
        ("repository_id", json!(1)),
        ("run_id", json!(0)),
        ("run_attempt", json!(0)),
        ("actor_id", json!(44)),
    ] {
        let mut v = serde_json::to_value(body(&f)).unwrap();
        v["controller"][field] = value;
        assert_eq!(
            reject(&f, &signed(&f, &serde_json::from_value(v).unwrap()), 100),
            IntentProjectionError::Principal
        );
    }
}

#[test]
fn collector_reserved_state_and_budget_grammar_cannot_be_relaxed() {
    for mutation in 0..8 {
        let mut f = Fixture::new(0);
        match mutation {
            0 => f.state.stage = DiagnosticStage::Starting,
            1 => f.state.stage = DiagnosticStage::Running,
            2 => f.state.stage = DiagnosticStage::Stopping,
            3 => f.state.stage = DiagnosticStage::Terminal,
            4 => f.state.admission_allowed = true,
            5 => f.state.completed_cells = 4,
            6 => f.state.last_observed_monotonic_ns = 12_000_000_000,
            _ => {
                f.state.last_observed_monotonic_ns = 301_000_000_000;
                f.state.controller_monotonic_ns = 301_000_000_000;
            }
        }
        assert_eq!(
            collector_unit_projection(&f.state),
            Err(IntentProjectionError::State)
        );
    }
    let f = Fixture::new(0);
    let original = project_collector_reserved_state(&f.state).unwrap();
    let mut late = f.state.clone();
    late.last_observed_monotonic_ns = 300_000_000_000;
    late.controller_monotonic_ns = 300_000_000_000;
    assert_ne!(
        original.unit_spec_sha256,
        project_collector_reserved_state(&late)
            .unwrap()
            .unit_spec_sha256
    );
}

#[test]
fn collector_original_guard_latches_packet_state_peer_and_time_drift() {
    for mutation in 0..6 {
        let mut f = Fixture::new(0);
        let raw = signed(&f, &body(&f));
        let mut worker = f.worker();
        let mut guard =
            verify_collector_intent_projection(&mut worker, &raw, &f.check(100)).unwrap();
        let original = f.state.clone();
        let uid = f.peer.uid;
        let gids = f.peer.gids.clone();
        let actors = f.controller.actor_ids.clone();
        match mutation {
            0 => f.state.revision += 1,
            1 => f.peer.uid = 901,
            2 => f.peer.gids.push(903),
            3 => f.controller.actor_ids.push(44),
            _ => (),
        }
        let altered = if mutation == 4 {
            signed(
                &f,
                &IntentProjectionBody {
                    request_id: "3".repeat(64),
                    ..body(&f)
                },
            )
        } else {
            raw.clone()
        };
        let error = guard
            .revalidate(&altered, &f.check(if mutation == 5 { 160 } else { 100 }))
            .unwrap_err();
        assert_eq!(
            error,
            match mutation {
                1..=3 => IntentProjectionError::Drift,
                5 => IntentProjectionError::Time,
                _ => IntentProjectionError::Binding,
            }
        );
        assert!(guard.is_refused());
        f.state = original;
        f.peer.uid = uid;
        f.peer.gids = gids;
        f.controller.actor_ids = actors;
        assert_eq!(
            guard.revalidate(&raw, &f.check(100)),
            Err(IntentProjectionError::Refused)
        );
        drop(guard);
        assert!(!worker.is_refused());
    }
}

#[test]
fn collector_original_worker_refusal_survives_guard_drop() {
    for mutation in 0..3 {
        let mut f = Fixture::new(0);
        let raw = signed(&f, &body(&f));
        let mut worker = f.worker();
        let mut guard =
            verify_collector_intent_projection(&mut worker, &raw, &f.check(100)).unwrap();
        let host = f.host.clone();
        let bytes = f.worker_bytes.clone();
        let trust = f.trust.clone();
        match mutation {
            0 => f.host.mount_namespace.inode += 1,
            1 => f.worker_bytes[0] ^= 1,
            _ => {
                f.trust = WorkerPolicyTrust::new(
                    SigningKey::from_bytes(&[74; 32]).verifying_key(),
                    &"a".repeat(64),
                    8,
                )
                .unwrap()
            }
        }
        assert!(matches!(
            guard.revalidate(&raw, &f.check(100)),
            Err(IntentProjectionError::Worker(_))
        ));
        drop(guard);
        f.host = host;
        f.worker_bytes = bytes;
        f.trust = trust;
        assert!(worker.is_refused());
        assert!(matches!(
            verify_collector_intent_projection(&mut worker, &raw, &f.check(100)),
            Err(IntentProjectionError::Worker(WorkerPolicyError::Refused))
        ));
    }
}

#[test]
fn collector_signed_field_mutations_are_not_production_authority() {
    let f = Fixture::new(0);
    let original = serde_json::to_value(body(&f)).unwrap();
    for (field, value) in [
        ("state_sha256", json!("3".repeat(64))),
        ("unit_spec_sha256", json!("4".repeat(64))),
        ("worker_policy_sha256", json!("5".repeat(64))),
        ("worker_policy_epoch", json!(8)),
        ("boot_id", json!("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")),
    ] {
        let mut v = original.clone();
        v[field] = value;
        assert_eq!(
            reject(&f, &signed(&f, &serde_json::from_value(v).unwrap()), 100),
            IntentProjectionError::Binding
        );
    }
}

#[test]
fn collector_seeded_full_state_binding_and_independent_concurrency() {
    const SEED: u64 = 0x0740c097;
    let mut seed = SEED;
    let f = Fixture::new(0);
    let initial = project_collector_reserved_state(&f.state).unwrap();
    for _ in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut state = f.state.clone();
        state.revision += 1 + seed % 4096;
        state.identity.build_provenance_sha256 = format!("{seed:064x}");
        let changed = project_collector_reserved_state(&state).unwrap();
        assert_ne!(initial.state_sha256, changed.state_sha256);
        assert_eq!(initial.unit_spec_sha256, changed.unit_spec_sha256);
        let bad = IntentProjectionBody {
            state_sha256: changed.state_sha256,
            ..body(&f)
        };
        assert_eq!(
            reject(&f, &signed(&f, &bad), 100),
            IntentProjectionError::Binding
        );
    }
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let tasks: Vec<_> = (0..4)
        .map(|cell| {
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let f = Fixture::new(cell);
                let raw = signed(&f, &body(&f));
                let mut worker = f.worker();
                let mut guard =
                    verify_collector_intent_projection(&mut worker, &raw, &f.check(100)).unwrap();
                barrier.wait();
                if cell == 0 {
                    assert_eq!(
                        guard.revalidate(&raw, &f.check(160)),
                        Err(IntentProjectionError::Time)
                    );
                } else {
                    guard.revalidate(&raw, &f.check(100)).unwrap();
                    assert!(!guard.is_refused());
                }
            })
        })
        .collect();
    for task in tasks {
        task.join().unwrap();
    }
    println!("collector_intent seed=0x{SEED:08x} state_mutations=64 independent_threads=4 production_start=false");
}
