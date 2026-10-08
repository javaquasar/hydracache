//! SYNTHETIC reports, build statement and test signer. No executable or timing.
use ed25519_dalek::{Signer, SigningKey};
use hydracache_long_run_supervisor_074::diagnostic_artifacts::*;
use hydracache_long_run_supervisor_074::diagnostic_lease::{
    CellIntent, DiagnosticIdentity, BINARY_PATH, SOURCE_COMMIT,
};
use hydracache_long_run_supervisor_074::diagnostic_receipts::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const CONFIGS: [&[u8]; 4] = [
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/embedded.json"),
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/direct.json"),
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/resp2.json"),
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/resp3.json"),
];
const SURFACES: [&str; 4] = ["embedded", "direct", "resp2", "resp3"];
const WORKLOADS: [&str; 4] = [
    "06d84110ec8fe56f986ef5cbef75b3e9df559449fdf32cb2c42680b683363a3c",
    "ee3dde1ca3322617ac08b6609bca5d151e9d49a8595a1743b3b3c321b22a1bc4",
    "e2ead8f4d1e61f1a1ab4246bb308ce7b00a2532bed52dc2c5ea1effb236fc11d",
    "642dd2671bdfb92c7d0d9400b10b9bd37dbbdd5b40ec2d0a50e8c4c1ea66c938",
];
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn bytes(v: &Value) -> Vec<u8> {
    let mut b = serde_json::to_vec(v).unwrap();
    b.push(b'\n');
    b
}

fn fixture(index: usize) -> (VerifiedBuild, CellIntent, TerminalSummary, Value) {
    // Only verifies an externally signed ASSERTION. No binary/log content here.
    let key = SigningKey::from_bytes(&[74; 32]);
    let statement = BuildStatement {
        schema_version: 1,
        repository_id: 74,
        builder_id: "synthetic-test-builder".into(),
        source_commit: SOURCE_COMMIT.into(),
        source_tree: SOURCE_TREE.into(),
        source_clean_before: true,
        source_clean_after: true,
        rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
        cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
        target: "x86_64-unknown-linux-gnu".into(),
        profile: "release".into(),
        features: vec![],
        allocator: "System".into(),
        counting_allocator: false,
        build_command: BUILD_COMMAND.iter().map(|s| s.to_string()).collect(),
        binary: ArtifactDigest::of(b"SYNTHETIC-NOT-AN-EXECUTABLE"),
        root_lock: ArtifactDigest::of(include_bytes!("../../../Cargo.lock")),
        observer_lock: ArtifactDigest::of(include_bytes!(
            "../../get-owner-scheduled-controls-074/Cargo.lock"
        )),
        build_log: ArtifactDigest::of(b"SYNTHETIC-NOT-A-COMPILER-LOG"),
        configs: BTreeMap::from(std::array::from_fn::<_, 4, _>(|i| {
            (SURFACES[i].into(), ArtifactDigest::of(CONFIGS[i]))
        })),
    };
    let signed = SignedBuild {
        signature_hex: hex(&key.sign(&signing_message(&statement).unwrap()).to_bytes()),
        statement,
    };
    let receipt = receipt_bytes(&signed).unwrap();
    let id = DiagnosticIdentity {
        lease_id: "a".repeat(64),
        boot_id: "12345678-1234-1234-1234-123456789abc".into(),
        binary_sha256: signed.statement.binary.sha256.clone(),
        build_provenance_sha256: hex(&Sha256::digest(&receipt)),
    };
    let build = verify_receipt(
        &receipt,
        &id,
        &BuildTrust {
            key: key.verifying_key(),
            repository_id: 74,
            builder_id: "synthetic-test-builder".into(),
        },
    )
    .unwrap();
    let unit = format!(
        "hydracache-diagnostic-074-{}-{}.service",
        id.lease_id,
        index + 1
    );
    let intent = CellIntent {
        lease_id: id.lease_id,
        boot_id: id.boot_id.clone(),
        surface: SURFACES[index].into(),
        unit_name: unit.clone(),
        cgroup_path: format!("/system.slice/{unit}"),
        binary_path: BINARY_PATH.into(),
        binary_sha256: id.binary_sha256.clone(),
        config_path: format!("{INSTALL_ROOT}/{}.json", SURFACES[index]),
        config_sha256: signed.statement.configs[SURFACES[index]].sha256.clone(),
        source_commit: SOURCE_COMMIT.into(),
        maximum_runtime_seconds: 60,
    };
    let terminal = TerminalSummary {
        unit_name: unit,
        boot_id: id.boot_id,
        cgroup_path: intent.cgroup_path.clone(),
        cgroup_inode: 74,
        populated: false,
        exit_code: Some(0),
        term_signal: None,
    };
    let input: Value = serde_json::from_slice(CONFIGS[index]).unwrap();
    let samples: Vec<_> = (0..10000u64).map(|i| json!({"sequence":i,"scheduled_ns":i*200000,"started_ns":i*200000+1000,
        "terminal_ns":i*200000+10000,"outcome":"success","scheduled_latency_ns":10000,"service_latency_ns":9000,"incomplete_lower_bound_ns":null})).collect();
    let latency = |v| {
        json!({"unit":"nanosecond-histogram-not-clock-resolution","samples":10000,
        "p50_ns":v,"p95_ns":v,"p99_ns":v,"overflow_count":0})
    };
    let operations = json!({"profile_id":"get-owner-scheduled-controls-074-v1","promotable":false,"product_performance_claim":false,
        "config":input["schedule"],"offered":10000,"target_started":10000,"target_completed":10000,"successes":10000,
        "errors":0,"timeouts":0,"queue_timeouts":0,"target_rejections":0,"admission_rejections":0,"incomplete":0,
        "late_successes":0,"good_successes":10000,"elapsed_ns":2000000000u64,"goodput_operations_per_second":5000.0,
        "good_fraction_of_all_offers":1.0,"pending_high_water":8,"execution_slots":8,"owned_tasks_drained":true,
        "scheduled_response_latency":latency(10007),"service_response_latency":latency(9007),"samples":samples});
    let observation = if index < 2 {
        json!({"boundary":"native","observed":operations})
    } else {
        let wires: Vec<_> = (0..8).flat_map(|c| (c..10000u64).step_by(8).map(move |i| json!({
            "dialect":SURFACES[index],"sequence":i,"connection_id":c,"wire_ordinal":i/8+100,
            "scheduled_ns":i*200000,"accepted_ns":i*200000+1000,"write_completed_ns":i*200000+2000,
            "response_complete_ns":i*200000+9000,"scheduled_frame_latency_ns":9000,"frame_kind":"bulk","response_items":1,
            "protocol_error_bytes":null,"byte_oracle_verified":true,"waiting_caller_cancelled":false,
            "transport_failure":null,"operation_outcome":"success"}))).collect();
        json!({"boundary":"resp","observed":{"dialect":SURFACES[index],"hello_connections":if index==3 {8} else {0},
            "hello_server_version":if index==3 {Some("0.74.0")} else {None},"operations":operations,"wire_samples":wires,
            "pipeline_limit":10,"physical_connections":8,"operation":"get","batch_size":1,"product_performance_claim":false,
            "authenticated_connections":0,"transport_security":null}})
    };
    let report = json!({"schema_version":1,"profile_id":"unprofiled-timing-controls-074-v1","input":input,
        "workload_sha256":WORKLOADS[index],"get_owner_feature":false,"runtime":"current-thread-required-by-executable",
        "allocator":"System-without-counting-wrapper-required-by-executable","admission_allowed":false,
        "product_numeric_claims_allowed":false,"cross_surface_numeric_comparison_allowed":false,
        "secure_fresh_process_material_parity_proven":false,"warmup_completed":64,"transport_security":null,
        "cpu":{"clock":{"provider":"CLOCK_PROCESS_CPUTIME_ID","scope":"whole-process-all-threads","unit_resolution_ns":1,
            "resolution_is_accuracy_claim":false},"process_cpu_ns":1000000000u64,"wall_elapsed_ns":2000000000u64,
            "cpu_ns_per_offer":100000.0,"cpu_ns_per_success":100000.0,"usable_for_ratio":true,"unusable_reasons":[],
            "scope":"whole-process scheduled-driver+byte-oracles+task/wire-drain+observation-projection; not server-only"},
        "observation":observation,"final_dataset_verified":true,"shutdown_verified":true,"error":null});
    let envelope = json!({"source_commit_from_coordinator":SOURCE_COMMIT,"source_git_identity_verified_by_binary":false,
        "binary_sha256":id.binary_sha256,"compiled_root_lock_sha256":ROOT_LOCK_SHA256,
        "compiled_observer_lock_sha256":OBSERVER_LOCK_SHA256,"report":report,"error":null});
    (build, intent, terminal, envelope)
}
fn decision(b: &VerifiedBuild, i: &CellIntent, t: &TerminalSummary, v: &Value) -> Decision {
    packet(b, i, t, &bytes(v), b"opaque diagnostic stderr\0")
        .unwrap()
        .decision
}

#[test]
fn all_four_p0_surfaces_bind_identity_and_reject_input_drift() {
    for index in 0..4 {
        let (b, i, t, v) = fixture(index);
        assert_eq!(
            decision(&b, &i, &t, &v),
            Decision::ValidCpuUsable,
            "{}",
            i.surface
        );
        for path in [
            "/source_commit_from_coordinator",
            "/binary_sha256",
            "/compiled_root_lock_sha256",
            "/compiled_observer_lock_sha256",
            "/report/workload_sha256",
            "/report/input/dataset_sha256",
        ] {
            let mut bad = v.clone();
            *bad.pointer_mut(path).unwrap() = json!("b".repeat(64));
            assert_eq!(
                decision(&b, &i, &t, &bad),
                Decision::InvalidContent,
                "{path}"
            );
        }
        for path in [
            "/report/input/seed",
            "/report/input/keyspace",
            "/report/input/payload_bytes",
            "/report/input/slots",
            "/report/input/pipeline_depth",
            "/report/input/warmup_calls",
            "/report/input/minimum_usable_cpu_ns",
            "/report/input/schedule/operations",
            "/report/input/schedule/slo_ns",
        ] {
            let mut bad = v.clone();
            let old = bad.pointer(path).unwrap().as_u64().unwrap();
            *bad.pointer_mut(path).unwrap() = json!(old + 1);
            assert_eq!(
                decision(&b, &i, &t, &bad),
                Decision::InvalidContent,
                "{path}"
            );
        }
        let mut other = i.clone();
        other.lease_id = "b".repeat(64);
        assert!(packet(&b, &other, &t, &bytes(&v), b"").is_err());
    }
}

#[test]
fn sample_accounting_latency_and_cpu_are_recomputed() {
    let (b, i, t, v) = fixture(0);
    for path in [
        "/report/observation/observed/offered",
        "/report/observation/observed/target_started",
        "/report/observation/observed/target_completed",
        "/report/observation/observed/successes",
        "/report/observation/observed/good_successes",
        "/report/observation/observed/late_successes",
        "/report/observation/observed/scheduled_response_latency/p95_ns",
        "/report/observation/observed/service_response_latency/p99_ns",
        "/report/observation/observed/samples/73/scheduled_ns",
        "/report/observation/observed/samples/73/sequence",
        "/report/observation/observed/samples/73/service_latency_ns",
        "/report/observation/observed/samples/73/terminal_ns",
        "/report/cpu/process_cpu_ns",
    ] {
        let mut bad = v.clone();
        let old = bad.pointer(path).unwrap().as_u64().unwrap();
        *bad.pointer_mut(path).unwrap() = json!(old + 1);
        assert_eq!(
            decision(&b, &i, &t, &bad),
            Decision::InvalidContent,
            "{path}"
        );
    }
    let mut bad_wall = v.clone();
    bad_wall["report"]["cpu"]["wall_elapsed_ns"] = json!(1);
    assert_eq!(decision(&b, &i, &t, &bad_wall), Decision::InvalidContent);
    for path in [
        "/report/observation/observed/goodput_operations_per_second",
        "/report/observation/observed/good_fraction_of_all_offers",
        "/report/cpu/cpu_ns_per_offer",
        "/report/cpu/cpu_ns_per_success",
    ] {
        let mut bad = v.clone();
        *bad.pointer_mut(path).unwrap() = json!(99.0);
        assert_eq!(
            decision(&b, &i, &t, &bad),
            Decision::InvalidContent,
            "{path}"
        );
    }
    let mut bad = v.clone();
    bad["report"]["observation"]["observed"]["samples"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(decision(&b, &i, &t, &bad), Decision::InvalidContent);
}

#[test]
fn resp_wire_requires_fifo_routing_and_byte_verified_success() {
    for index in [2, 3] {
        let (b, i, t, v) = fixture(index);
        for (path, value) in [
            (
                "/report/observation/observed/wire_samples/1/wire_ordinal",
                json!(100),
            ),
            (
                "/report/observation/observed/wire_samples/1/sequence",
                json!(0),
            ),
            (
                "/report/observation/observed/wire_samples/0/connection_id",
                json!(1),
            ),
            (
                "/report/observation/observed/wire_samples/0/response_complete_ns",
                json!(10001),
            ),
            (
                "/report/observation/observed/wire_samples/0/write_completed_ns",
                json!(99999),
            ),
            (
                "/report/observation/observed/wire_samples/0/byte_oracle_verified",
                json!(false),
            ),
            (
                "/report/observation/observed/wire_samples/0/waiting_caller_cancelled",
                json!(true),
            ),
            (
                "/report/observation/observed/wire_samples/0/frame_kind",
                json!("null"),
            ),
            (
                "/report/observation/observed/wire_samples/0/response_items",
                json!(2),
            ),
            (
                "/report/observation/observed/wire_samples/0/transport_failure",
                json!("disconnect"),
            ),
            ("/report/observation/observed/hello_connections", json!(99)),
            (
                "/report/observation/observed/authenticated_connections",
                json!(8),
            ),
            ("/report/observation/observed/pipeline_limit", json!(50)),
        ] {
            let mut bad = v.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert_eq!(
                decision(&b, &i, &t, &bad),
                Decision::InvalidContent,
                "{path}"
            );
        }
        let mut bad = v.clone();
        bad["report"]["observation"]["observed"]["wire_samples"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert_eq!(decision(&b, &i, &t, &bad), Decision::InvalidContent);
    }
}

#[test]
fn failed_incomplete_and_cpu_unusable_attempts_remain_distinct() {
    let (b, i, mut t, mut v) = fixture(0);
    v["report"]["cpu"]["process_cpu_ns"] = json!(999990000);
    v["report"]["cpu"]["cpu_ns_per_offer"] = json!(99999.0);
    v["report"]["cpu"]["cpu_ns_per_success"] = json!(99999.0);
    v["report"]["cpu"]["usable_for_ratio"] = json!(false);
    v["report"]["cpu"]["unusable_reasons"] = json!(["CPU-below-predeclared-minimum"]);
    assert_eq!(decision(&b, &i, &t, &v), Decision::ValidCpuUnusable);
    v["report"]["cpu"]["usable_for_ratio"] = json!(true);
    assert_eq!(decision(&b, &i, &t, &v), Decision::InvalidContent);
    v["report"]["error"] = json!("synthetic timeout/disconnect; retained, never retried");
    assert_eq!(decision(&b, &i, &t, &v), Decision::FailedReport);
    v["report"] = Value::Null;
    v["error"] = json!("process fixture deadline; no shutdown proof");
    assert_eq!(decision(&b, &i, &t, &v), Decision::FailedReport);
    t.exit_code = Some(1);
    assert_eq!(
        packet(&b, &i, &t, b"", b"synthetic startup failure")
            .unwrap()
            .decision,
        Decision::FailedProcess
    );
    let (_, _, _, valid) = fixture(0);
    assert_eq!(decision(&b, &i, &t, &valid), Decision::FailedProcess);
    t.populated = true;
    assert_eq!(decision(&b, &i, &t, &valid), Decision::TerminalUnproven);
    t.populated = false;
    t.cgroup_inode = 0;
    assert_eq!(decision(&b, &i, &t, &valid), Decision::TerminalUnproven);
}

#[test]
fn malformed_duplicate_unknown_missing_and_oversized_bytes_fail_closed() {
    let (b, i, t, v) = fixture(0);
    let good = bytes(&v);
    for malformed in [
        good[..good.len() - 1].to_vec(),
        good[..73].to_vec(),
        b"{}\n{}\n".to_vec(),
        b"{\"error\":null,\"error\":\"override\"}\n".to_vec(),
        b"{\"x\":NaN}\n".to_vec(),
        b"{\"x\":1e999}\n".to_vec(),
    ] {
        let p = packet(&b, &i, &t, &malformed, b"failure retained").unwrap();
        assert_eq!(p.decision, Decision::InvalidContent);
        assert_eq!(p.stdout, ArtifactDigest::of(&malformed));
    }
    for path in [
        "/report",
        "/report/cpu",
        "/report/observation/observed/samples/0",
    ] {
        let mut bad = v.clone();
        bad.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(1));
        assert_eq!(decision(&b, &i, &t, &bad), Decision::InvalidContent);
    }
    let mut bad = v.clone();
    bad["report"].as_object_mut().unwrap().remove("error");
    assert_eq!(decision(&b, &i, &t, &bad), Decision::InvalidContent);
    let text = String::from_utf8(good.clone()).unwrap().replace(
        "\"process_cpu_ns\":1000000000",
        "\"process_cpu_ns\":1000000000,\"process_cpu_ns\":1",
    );
    assert_ne!(text.as_bytes(), good);
    assert_eq!(
        packet(&b, &i, &t, text.as_bytes(), b"").unwrap().decision,
        Decision::InvalidContent
    );
    assert!(matches!(
        packet(&b, &i, &t, &vec![0; STREAM_BYTES + 1], b""),
        Err(ReceiptError::Overflow)
    ));
    assert!(matches!(
        packet(&b, &i, &t, &good, &vec![0; STREAM_BYTES + 1]),
        Err(ReceiptError::Overflow)
    ));
    assert_eq!(
        packet(&b, &i, &t, &good, &vec![0; STREAM_BYTES])
            .unwrap()
            .stderr
            .bytes,
        STREAM_BYTES as u64
    );
}

#[test]
fn packet_replay_recomputes_decision_and_binds_original_bytes() {
    let (b, i, t, v) = fixture(0);
    let raw = bytes(&v);
    let stderr = b"opaque stderr\0\xff";
    let p = packet(&b, &i, &t, &raw, stderr).unwrap();
    let sealed = packet_bytes(&p).unwrap();
    assert_eq!(verify_packet(&sealed, &b, &i, &t, &raw, stderr).unwrap(), p);
    for flag in [
        "promotable",
        "admission_allowed",
        "product_numeric_claims_allowed",
        "durable_spool_proven",
        "live_cgroup_proven",
    ] {
        let mut bad = serde_json::to_value(&p).unwrap();
        bad[flag] = json!(true);
        assert!(verify_packet(&bytes(&bad), &b, &i, &t, &raw, stderr).is_err());
    }
    let mut bad = serde_json::to_value(&p).unwrap();
    bad["decision"] = json!("valid-cpu-unusable");
    assert!(verify_packet(&bytes(&bad), &b, &i, &t, &raw, stderr).is_err());
    assert!(verify_packet(&sealed, &b, &i, &t, &raw, b"changed stderr").is_err());
    let mut spaced = raw.clone();
    spaced.insert(0, b' ');
    assert_eq!(
        packet(&b, &i, &t, &spaced, stderr).unwrap().decision,
        Decision::ValidCpuUsable
    );
    assert!(verify_packet(&sealed, &b, &i, &t, &spaced, stderr).is_err());
    let mut other = t.clone();
    other.cgroup_inode += 1;
    assert!(verify_packet(&sealed, &b, &i, &other, &raw, stderr).is_err());
}

#[test]
fn concurrent_offline_readers_return_identical_non_promotable_packets() {
    let (b, i, t, v) = fixture(0);
    let raw = bytes(&v);
    let expected = packet_bytes(&packet(&b, &i, &t, &raw, b"").unwrap()).unwrap();
    std::thread::scope(|scope| {
        let readers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let checked = verify_packet(&expected, &b, &i, &t, &raw, b"").unwrap();
                    assert!(
                        !checked.promotable
                            && !checked.durable_spool_proven
                            && !checked.live_cgroup_proven
                    );
                    packet_bytes(&checked).unwrap()
                })
            })
            .collect();
        for reader in readers {
            assert_eq!(reader.join().unwrap(), expected);
        }
    });
}

#[test]
fn claim_flags_wrong_boundaries_and_uncertain_terminal_never_pass() {
    let (b, i, t, v) = fixture(0);
    for path in [
        "/source_git_identity_verified_by_binary",
        "/report/get_owner_feature",
        "/report/admission_allowed",
        "/report/product_numeric_claims_allowed",
        "/report/cross_surface_numeric_comparison_allowed",
        "/report/secure_fresh_process_material_parity_proven",
        "/report/cpu/clock/resolution_is_accuracy_claim",
        "/report/observation/observed/promotable",
    ] {
        let mut bad = v.clone();
        *bad.pointer_mut(path).unwrap() = json!(true);
        assert_eq!(
            decision(&b, &i, &t, &bad),
            Decision::InvalidContent,
            "{path}"
        );
    }
    for change in 0..5 {
        let mut terminal = t.clone();
        match change {
            0 => terminal.boot_id = "22345678-1234-1234-1234-123456789abc".into(),
            1 => terminal.unit_name = "other.service".into(),
            2 => terminal.exit_code = None,
            3 => terminal.term_signal = Some(9),
            _ => terminal.cgroup_path = "/system.slice/other.service".into(),
        }
        assert_eq!(decision(&b, &i, &terminal, &v), Decision::TerminalUnproven);
    }
    let mut oversized = t.clone();
    oversized.unit_name = "x".repeat(257);
    assert!(packet(&b, &i, &oversized, &bytes(&v), b"").is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn synthetic_valid_report_survives_readonly_spool_and_offline_verification() {
    use hydracache_long_run_supervisor_074::diagnostic_spool::{
        inspect_fixture_spool, verify_fixture_packet,
    };
    use std::os::unix::fs::PermissionsExt;
    let (b, i, t, v) = fixture(0);
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let output = temp.path().join("output");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&output).unwrap();
    for dir in [&source, &output] {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let raw = bytes(&v);
    std::fs::write(source.join("stdout.json"), &raw).unwrap();
    std::fs::write(source.join("stderr.log"), b"opaque").unwrap();
    for name in ["stdout.json", "stderr.log"] {
        std::fs::set_permissions(source.join(name), std::fs::Permissions::from_mode(0o600))
            .unwrap();
    }
    // SAFETY: effective local fixture identity only.
    let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let mut spool = inspect_fixture_spool(&source, uid, gid).unwrap();
    let p = spool.publish_fixture(&output, &b, &i, &t, None).unwrap();
    assert_eq!(
        p.manifest().packet.as_ref().unwrap().decision,
        Decision::ValidCpuUsable
    );
    let checked =
        verify_fixture_packet(p.path(), uid, gid, &b, &i, &t, p.manifest_digest()).unwrap();
    assert_eq!(&checked, p.manifest());
    assert_eq!(std::fs::read(p.path().join("stdout.prefix")).unwrap(), raw);
    assert!(!checked.live_cgroup_proven && !checked.admission_allowed && checked.fixture_only);
    std::fs::set_permissions(p.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
}
