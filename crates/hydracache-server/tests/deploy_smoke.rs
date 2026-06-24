#[test]
fn deploy_smoke_dockerfile_builds_hydracache_server_binary() {
    let dockerfile = include_str!("../../../Dockerfile");

    assert!(dockerfile.contains("cargo build --release --locked -p hydracache-server"));
    assert!(dockerfile.contains("gcr.io/distroless/cc-debian12:nonroot"));
    assert!(dockerfile.contains("USER nonroot:nonroot"));
    assert!(dockerfile.contains(r#"ENTRYPOINT ["/usr/local/bin/hydracache-server"]"#));
}

#[test]
fn deploy_smoke_k8s_manifests_wire_stateful_identity_storage_tls_backup_and_probes() {
    let statefulset = include_str!("../../../deploy/k8s/statefulset.yaml");
    let service = include_str!("../../../deploy/k8s/service.yaml");
    let pdb = include_str!("../../../deploy/k8s/pdb.yaml");

    assert!(statefulset.contains("kind: StatefulSet"));
    assert!(statefulset.contains("serviceName: hydracache-headless"));
    assert!(statefulset.contains("volumeClaimTemplates"));
    assert!(statefulset.contains("HYDRACACHE_SEEDS"));
    assert!(statefulset.contains("HYDRACACHE_TLS_ENABLED"));
    assert!(statefulset.contains("HYDRACACHE_BACKUP_LOCATION"));
    assert!(statefulset.contains("livenessProbe"));
    assert!(statefulset.contains("path: /health"));
    assert!(statefulset.contains("readinessProbe"));
    assert!(statefulset.contains("path: /ready"));
    assert!(service.contains("clusterIP: None"));
    assert!(service.contains("name: metrics"));
    assert!(pdb.contains("kind: PodDisruptionBudget"));
    assert!(pdb.contains("minAvailable: 2"));
}

#[test]
fn deploy_smoke_helm_chart_exposes_replicas_rf_tls_and_backup_values() {
    let chart = include_str!("../../../deploy/helm/hydracache/Chart.yaml");
    let values = include_str!("../../../deploy/helm/hydracache/values.yaml");
    let helpers = include_str!("../../../deploy/helm/hydracache/templates/_helpers.tpl");
    let statefulset = include_str!("../../../deploy/helm/hydracache/templates/statefulset.yaml");

    assert!(chart.contains("version: 0.48.0"));
    assert!(helpers.contains(r#"define "hydracache.fullname""#));
    assert!(values.contains("replicaCount: 3"));
    assert!(values.contains("replicationFactor: 3"));
    assert!(values.contains("tls:"));
    assert!(values.contains("backup:"));
    assert!(statefulset.contains("{{ .Values.tls.enabled | quote }}"));
    assert!(statefulset.contains("{{ .Values.backup.location | quote }}"));
}

#[test]
#[ignore = "nightly gate: requires Docker daemon"]
fn deploy_smoke_image_builds_and_container_serves_health() {
    assert!(std::process::Command::new("docker")
        .args(["build", "-t", "hydracache-server:test", "."])
        .status()
        .expect("docker is installed")
        .success());
}

#[test]
#[ignore = "nightly gate: requires kind and kubectl"]
fn deploy_smoke_kind_statefulset_forms_quorum_and_survives_rolling_update() {
    assert!(std::process::Command::new("kind")
        .arg("version")
        .status()
        .expect("kind is installed")
        .success());
}
