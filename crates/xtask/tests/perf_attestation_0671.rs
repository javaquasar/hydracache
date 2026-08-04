use hydracache_loadgen::profile::{
    reference_cpu_isolation, MeasurementCore, PerformanceProfile, RunnerAttestationV5,
    RunnerFingerprint, REFERENCE_FINGERPRINT_SCHEMA_VERSION, REFERENCE_HOST_CONTRACT_VERSION,
    REFERENCE_MEASUREMENT_CPUS, REFERENCE_OS_IMAGE, REFERENCE_RUNNER_CLASS,
    REFERENCE_STORAGE_CLASS,
};

fn attestation() -> RunnerAttestationV5 {
    RunnerAttestationV5 {
        schema_version: REFERENCE_FINGERPRINT_SCHEMA_VERSION,
        contract_version: REFERENCE_HOST_CONTRACT_VERSION.to_owned(),
        virtualization: "none".to_owned(),
        physical_cores: 8,
        measurement_cores: REFERENCE_MEASUREMENT_CPUS
            .into_iter()
            .map(|logical_cpu| MeasurementCore {
                logical_cpu,
                package_id: 0,
                core_id: logical_cpu,
            })
            .collect(),
        cpu_isolation: reference_cpu_isolation(),
        host_digest: "a".repeat(64),
        storage_class: REFERENCE_STORAGE_CLASS.to_owned(),
        storage_identity_digest: "b".repeat(64),
        os_image: REFERENCE_OS_IMAGE.to_owned(),
        toolchain_identity: "rustc-1.94.0".to_owned(),
        prebuild_contract_digest: "c".repeat(64),
    }
}

fn fingerprint(attestation: RunnerAttestationV5) -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        fingerprint: "d".repeat(64),
        cpu_model: "fixture physical cpu".to_owned(),
        logical_cores: 16,
        ram_bytes: 64 * 1024 * 1024 * 1024,
        os: "linux".to_owned(),
        kernel: "fixture-kernel".to_owned(),
        cpu_affinity: "1-4".to_owned(),
        cgroup_cpu_quota: "unlimited".to_owned(),
        governor: "performance".to_owned(),
        turbo: "disabled".to_owned(),
        shared_hardware: false,
        calibration_score: 0.01,
        attestation,
    }
}

fn profile() -> PerformanceProfile {
    PerformanceProfile {
        name: "reference-v1".to_owned(),
        required_runner_class: REFERENCE_RUNNER_CLASS.to_owned(),
        allowed_fingerprints: Vec::new(),
        minimum_logical_cores: 4,
        required_cpu_affinity: "1-4".to_owned(),
        required_cgroup_cpu_quota: "unlimited".to_owned(),
        require_dedicated: true,
        maximum_calibration_score: 0.25,
    }
}

fn rejected(mutator: impl FnOnce(&mut RunnerAttestationV5)) -> bool {
    let mut observed = attestation();
    mutator(&mut observed);
    !profile().validate(&fingerprint(observed)).eligible
}

#[test]
fn reference_attestation_v5_rejects_vm_siblings_non_nvme_and_missing_identity() {
    assert!(profile().validate(&fingerprint(attestation())).eligible);
    assert!(rejected(|value| value.virtualization = "kvm".to_owned()));
    assert!(rejected(|value| {
        value.measurement_cores[3].core_id = value.measurement_cores[2].core_id;
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.smt_control = "on".to_owned();
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.isolated_cpus.clear();
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.irq_affinity_policy = "shared".to_owned();
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.measurement_idle_policy = "unrestricted".to_owned();
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.measurement_max_idle_latency_us = 400;
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.housekeeping_idle_policy = "unrestricted".to_owned();
    }));
    assert!(rejected(|value| {
        value.cpu_isolation.housekeeping_max_idle_latency_us = 400;
    }));
    assert!(rejected(|value| value.schema_version = 4));
    assert!(rejected(
        |value| value.contract_version = "hydracache-reference-host-v4".to_owned()
    ));
    assert!(rejected(|value| {
        value.storage_class = "network-block".to_owned();
    }));
    assert!(rejected(|value| value.host_digest.clear()));
    assert!(rejected(|value| value.storage_identity_digest.clear()));
    assert!(rejected(|value| value.os_image = "ubuntu-22.04".to_owned()));
    assert!(rejected(|value| value.toolchain_identity.clear()));
    assert!(rejected(|value| value.prebuild_contract_digest.clear()));
}

#[test]
fn fingerprint_v5_serializes_only_privacy_safe_bound_attestation() {
    let observed = fingerprint(attestation());
    let json = serde_json::to_string(&observed).unwrap();
    assert!(json.contains(REFERENCE_HOST_CONTRACT_VERSION));
    assert!(json.contains(REFERENCE_STORAGE_CLASS));
    assert!(json.contains("rustc-1.94.0"));
    assert!(!json.contains("product_uuid"));
    assert!(!json.contains("board_serial"));
    assert!(!json.contains("disk_serial"));
}

#[test]
fn canary_attestation_accepts_a_virtualized_reference_host() {
    let accepted = !rejected(|value| value.virtualization = "kvm".to_owned());
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W2") {
        assert!(
            accepted,
            "HC-CANARY-RED:W2 virtualized reference host reached the measurement lane"
        );
    } else {
        assert!(!accepted);
    }
}
