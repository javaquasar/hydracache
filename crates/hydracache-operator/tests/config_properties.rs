mod config_properties {
    use hydracache_operator::crd::{sample_spec, HydraCacheCluster};
    use hydracache_operator::resources::{
        OwnedResources, ADMIN_PORT, CLIENT_PORT, CLUSTER_PORT, DATA_VOLUME, TLS_VOLUME,
    };
    use k8s_openapi::api::apps::v1::StatefulSet;
    use k8s_openapi::api::core::v1::{Secret, Service};
    use proptest::prelude::*;
    use proptest::test_runner::TestRunner;

    #[test]
    fn operator_manifest_roundtrip_preserves_security_and_storage_contract() {
        let strategy = (
            "[a-z][a-z0-9]{2,16}",
            1_u32..8,
            1_u32..128,
            "[a-z][a-z0-9]{2,12}",
        );
        let mut runner = TestRunner::default();
        runner
            .run(&strategy, |(suffix, replicas, storage_gib, tls_suffix)| {
                let name = format!("matrix-{suffix}");
                let mut spec = sample_spec();
                spec.replicas = replicas;
                spec.persistence.as_mut().unwrap().size = format!("{storage_gib}Gi");
                spec.tls.as_mut().unwrap().secret_name = format!("tls-{tls_suffix}");
                let tls_secret_name = spec.tls.as_ref().unwrap().secret_name.clone();
                let mut cluster = HydraCacheCluster::new(&name, spec);
                cluster.metadata.namespace = Some("matrix-system".to_owned());
                cluster.metadata.uid = Some(format!("uid-{suffix}"));

                let resources = OwnedResources::build(&cluster);
                let stateful_json = serde_json::to_string(&resources.stateful_set).unwrap();
                let stateful: StatefulSet = serde_json::from_str(&stateful_json).unwrap();
                prop_assert_eq!(&stateful, &resources.stateful_set);
                let headless: Service = serde_json::from_str(
                    &serde_json::to_string(&resources.headless_service).unwrap(),
                )
                .unwrap();
                let client: Service = serde_json::from_str(
                    &serde_json::to_string(&resources.client_service).unwrap(),
                )
                .unwrap();
                let admin: Secret =
                    serde_json::from_str(&serde_json::to_string(&resources.admin_secret).unwrap())
                        .unwrap();
                prop_assert_eq!(&headless, &resources.headless_service);
                prop_assert_eq!(&client, &resources.client_service);
                prop_assert_eq!(&admin, &resources.admin_secret);

                prop_assert_eq!(stateful.metadata.name.as_deref(), Some(name.as_str()));
                prop_assert_eq!(
                    stateful.metadata.namespace.as_deref(),
                    Some("matrix-system")
                );
                let owner = &stateful.metadata.owner_references.as_ref().unwrap()[0];
                prop_assert_eq!(owner.name.as_str(), name.as_str());
                prop_assert_eq!(owner.uid.as_str(), format!("uid-{suffix}"));

                let stateful_spec = stateful.spec.as_ref().unwrap();
                prop_assert_eq!(stateful_spec.replicas, Some(replicas as i32));
                let claim = &stateful_spec.volume_claim_templates.as_ref().unwrap()[0];
                prop_assert_eq!(claim.metadata.name.as_deref(), Some(DATA_VOLUME));
                let request = &claim
                    .spec
                    .as_ref()
                    .unwrap()
                    .resources
                    .as_ref()
                    .unwrap()
                    .requests
                    .as_ref()
                    .unwrap()["storage"]
                    .0;
                prop_assert_eq!(request, &format!("{storage_gib}Gi"));

                let pod = stateful_spec.template.spec.as_ref().unwrap();
                let container = &pod.containers[0];
                let ports = container.ports.as_ref().unwrap();
                for required in [CLIENT_PORT, CLUSTER_PORT, ADMIN_PORT] {
                    prop_assert!(ports.iter().any(|port| port.container_port == required));
                }
                prop_assert_eq!(
                    container
                        .readiness_probe
                        .as_ref()
                        .unwrap()
                        .http_get
                        .as_ref()
                        .unwrap()
                        .path
                        .as_deref(),
                    Some("/readyz")
                );
                prop_assert_eq!(
                    container
                        .liveness_probe
                        .as_ref()
                        .unwrap()
                        .http_get
                        .as_ref()
                        .unwrap()
                        .path
                        .as_deref(),
                    Some("/healthz")
                );
                let tls_volume = pod
                    .volumes
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|volume| volume.name == TLS_VOLUME)
                    .unwrap();
                prop_assert_eq!(
                    tls_volume.secret.as_ref().unwrap().secret_name.as_deref(),
                    Some(tls_secret_name.as_str())
                );
                let has_read_only_tls_mount = container
                    .volume_mounts
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|mount| mount.name == TLS_VOLUME && mount.read_only == Some(true));
                prop_assert!(has_read_only_tls_mount);

                let env = container.env.as_ref().unwrap();
                let env_value = |name: &str| {
                    env.iter()
                        .find(|entry| entry.name == name)
                        .and_then(|entry| entry.value.as_deref())
                };
                prop_assert_eq!(env_value("HYDRACACHE_TLS_ENABLED"), Some("true"));
                prop_assert_eq!(env_value("HYDRACACHE_TLS_ACK_INSECURE"), Some("false"));
                prop_assert_eq!(
                    env_value("HYDRACACHE_TLS_KEY_PATH"),
                    Some("/etc/hydracache/tls/tls.key")
                );
                Ok(())
            })
            .unwrap();
    }
}
