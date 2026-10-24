use super::*;
use crate::diagnostic_artifacts::{BuildStatement, SignedBuild};
use crate::diagnostic_lease::DiagnosticStage;

fn identity() -> DiagnosticIdentity {
    DiagnosticIdentity {
        lease_id: "a".repeat(64),
        boot_id: "12345678-1234-1234-1234-123456789abc".into(),
        binary_sha256: "b".repeat(64),
        build_provenance_sha256: "c".repeat(64),
    }
}
fn state() -> DiagnosticState {
    DiagnosticState {
        identity: identity(),
        revision: 1,
        stage: DiagnosticStage::Reserved,
        completed_cells: 0,
        reserved_monotonic_ns: 1_000_000_000,
        last_observed_monotonic_ns: 1_000_000_000,
        controller_monotonic_ns: 1_000_000_000,
        cell_started_monotonic_ns: None,
        cgroup_inode: None,
        reason: None,
        cleanup_confirmed: true,
        promotable: false,
        admission_allowed: false,
    }
}
#[test]
fn original_start_material_requirements_retain_collector_not_legacy_environment() {
    let state = state();
    let raw = collector_unit_projection(&state).unwrap();
    assert_eq!(
        requirements(
            &state,
            &crate::sha256_hex(&raw),
            &crate::sha256_hex(&crate::canonical_json(&state).unwrap())
        )
        .unwrap(),
        raw
    );
    let text = String::from_utf8(raw).unwrap();
    assert!(text.contains("original-owned-pipes"));
    assert!(text.contains("sole-root-collector"));
    assert!(text.contains("PATH=/usr/bin:/bin"));
    assert!(text.contains("LC_ALL=C"));
    assert!(text.contains("all-creator-manager-writer-copies-closed"));
    assert!(!text.contains("StandardOutputFileToAppend"));
    assert!(!text.contains("StandardErrorFileToAppend"));
    assert!(!text.contains("RUNNER_TRACKING_ID"));
}
#[test]
fn original_start_material_requirements_refuse_digest_state_and_seeded_mutations() {
    let state = state();
    let digest = crate::sha256_hex(&collector_unit_projection(&state).unwrap());
    let state_digest = crate::sha256_hex(&crate::canonical_json(&state).unwrap());
    let mut seed = 0x0740_5221_u64;
    for _ in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut bytes = digest.as_bytes().to_vec();
        let i = seed as usize % bytes.len();
        bytes[i] = if bytes[i] == b'0' { b'1' } else { b'0' };
        assert!(matches!(
            requirements(&state, &String::from_utf8(bytes).unwrap(), &state_digest),
            Err(OriginalStartMaterialError::Binding)
        ));
    }
    for kind in 0..5 {
        let mut altered = state.clone();
        match kind {
            0 => altered.stage = DiagnosticStage::Starting,
            1 => altered.revision = 2,
            2 => altered.completed_cells = 1,
            3 => altered.identity.binary_sha256 = "d".repeat(64),
            4 => altered.promotable = true,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                requirements(&altered, &digest, &state_digest),
                Err(OriginalStartMaterialError::Binding)
            ),
            "kind={kind}"
        );
    }
    println!("HC074-START-MATERIAL-REQUIREMENTS seed=0x07405221 mutations=256 states=5 exact=true");
}
#[test]
fn original_start_material_equal_build_requires_every_statement_field() {
    let raw = include_bytes!("../../../docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/build-receipt-v1.json");
    let doc: SignedBuild = serde_json::from_slice(raw).unwrap();
    let expected = doc.statement;
    let identity = identity();
    equal_build(&identity, &expected, &identity, &expected).unwrap();
    let value = serde_json::to_value(&expected).unwrap();
    for key in value.as_object().unwrap().keys() {
        let mut changed = value.clone();
        let field = &mut changed[key];
        match field {
            serde_json::Value::String(s) => s.push('x'),
            serde_json::Value::Bool(b) => *b = !*b,
            serde_json::Value::Number(n) => *n = (n.as_u64().unwrap() + 1).into(),
            serde_json::Value::Array(v) => v.push(serde_json::json!("different")),
            serde_json::Value::Object(v) => {
                if let Some(hash) = v.get_mut("sha256") {
                    *hash = serde_json::json!("f".repeat(64));
                } else {
                    v.values_mut().next().unwrap()["sha256"] = serde_json::json!("e".repeat(64));
                }
            }
            serde_json::Value::Null => panic!("unexpected null"),
        }
        let changed: BuildStatement = serde_json::from_value(changed).unwrap();
        {
            assert!(
                matches!(
                    equal_build(&identity, &expected, &identity, &changed),
                    Err(OriginalStartMaterialError::Binding)
                ),
                "key={key}"
            );
        }
    }
    for kind in 0..4 {
        let mut changed = identity.clone();
        match kind {
            0 => changed.lease_id.push('x'),
            1 => changed.boot_id.push('x'),
            2 => changed.binary_sha256.push('x'),
            3 => changed.build_provenance_sha256.push('x'),
            _ => unreachable!(),
        }
        assert!(matches!(
            equal_build(&identity, &expected, &changed, &expected),
            Err(OriginalStartMaterialError::Binding)
        ));
    }
}
