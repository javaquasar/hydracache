use hydracache_operator::crd::{
    sample_spec, HydraCacheCluster, HYDRACACHE_CLUSTER_CRD_NAME, HYDRACACHE_CLUSTER_PLURAL,
    HYDRACACHE_GROUP, HYDRACACHE_VERSION,
};
use kube::core::CustomResourceExt;
use serde_json::Value;

fn crd_json() -> Value {
    serde_json::to_value(HydraCacheCluster::crd()).expect("CRD serializes to JSON")
}

fn find_property<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    if let Some(property) = value
        .get("properties")
        .and_then(|properties| properties.get(name))
    {
        return Some(property);
    }
    match value {
        Value::Array(items) => items.iter().find_map(|item| find_property(item, name)),
        Value::Object(map) => map.values().find_map(|item| find_property(item, name)),
        _ => None,
    }
}

#[test]
fn crd_malformed_spec_is_rejected_by_validation() {
    let crd = crd_json();
    assert_eq!(crd["metadata"]["name"], HYDRACACHE_CLUSTER_CRD_NAME);
    assert_eq!(crd["spec"]["group"], HYDRACACHE_GROUP);
    assert_eq!(crd["spec"]["names"]["plural"], HYDRACACHE_CLUSTER_PLURAL);
    assert_eq!(crd["spec"]["versions"][0]["name"], HYDRACACHE_VERSION);

    assert_eq!(find_property(&crd, "replicas").unwrap()["minimum"], 1.0);
    assert_eq!(find_property(&crd, "image").unwrap()["minLength"], 1);
    assert_eq!(find_property(&crd, "version").unwrap()["minLength"], 1);
    assert_eq!(find_property(&crd, "regions").unwrap()["minItems"], 1);
    assert_eq!(find_property(&crd, "secretName").unwrap()["minLength"], 1);
    assert_eq!(
        find_property(&crd, "storageClassName").unwrap()["minLength"],
        1
    );

    let subresources = &crd["spec"]["versions"][0]["subresources"];
    assert_eq!(subresources["scale"]["specReplicasPath"], ".spec.replicas");
    assert_eq!(
        subresources["scale"]["statusReplicasPath"],
        ".status.observedReplicas"
    );
    assert!(subresources.get("status").is_some());
}

#[test]
fn crd_manifest_records_generated_surface() {
    let manifest =
        include_str!("../../../deploy/operator/hydracacheclusters.hydracache.io.crd.yaml");
    assert!(manifest.contains("kind: CustomResourceDefinition"));
    assert!(manifest.contains("name: hydracacheclusters.hydracache.io"));
    assert!(manifest.contains("group: hydracache.io"));
    assert!(manifest.contains("shortNames:"));
    assert!(manifest.contains("- hcc"));
    assert!(manifest.contains("specReplicasPath: .spec.replicas"));
    assert!(manifest.contains("statusReplicasPath: .status.observedReplicas"));
    assert!(manifest.contains("minLength: 1"));
    assert!(manifest.contains("minimum: 1"));
}

#[test]
fn crd_rbac_is_least_privilege_no_cluster_admin() {
    let rbac = include_str!("../../../deploy/operator/rbac.yaml");
    assert!(rbac.contains("kind: ServiceAccount"));
    assert!(rbac.contains("kind: Role"));
    assert!(rbac.contains("kind: RoleBinding"));
    assert!(!rbac.contains("cluster-admin"));
    assert!(!rbac.contains("verbs: [\"*\"]"));
    assert!(!rbac.contains("resources: [\"*\"]"));
    assert!(rbac.contains("resources: [\"statefulsets\"]"));
    assert!(rbac.contains("resources: [\"services\", \"secrets\", \"persistentvolumeclaims\"]"));
    assert!(rbac.contains("resources: [\"pods\"]"));
    assert!(rbac.contains("resources: [\"hydracacheclusters\"]"));
    assert!(rbac
        .contains("resources: [\"hydracacheclusters/status\", \"hydracacheclusters/finalizers\"]"));
    assert!(rbac.contains("resources: [\"leases\"]"));
}

#[tokio::test]
async fn crd_roundtrips_apply_get() {
    if std::env::var("HYDRACACHE_OPERATOR_ENVTEST").as_deref() != Ok("1") {
        eprintln!(
            "skipping envtest apply/get: set HYDRACACHE_OPERATOR_ENVTEST=1 with a test apiserver"
        );
        return;
    }

    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
    use kube::api::{Api, Patch, PatchParams};

    let client = kube::Client::try_default()
        .await
        .expect("HYDRACACHE_OPERATOR_ENVTEST=1 requires kube config/envtest apiserver");
    let apply = PatchParams::apply("hydracache-operator-tests").force();
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    crds.patch(
        HYDRACACHE_CLUSTER_CRD_NAME,
        &apply,
        &Patch::Apply(&HydraCacheCluster::crd()),
    )
    .await
    .expect("CRD applies to envtest apiserver");

    let clusters: Api<HydraCacheCluster> = Api::namespaced(client, "default");
    let desired = HydraCacheCluster::new("roundtrip-w1", sample_spec());
    let applied = clusters
        .patch("roundtrip-w1", &apply, &Patch::Apply(&desired))
        .await
        .expect("HydraCacheCluster applies");
    let fetched = clusters
        .get("roundtrip-w1")
        .await
        .expect("HydraCacheCluster can be read back");

    assert_eq!(applied.spec, fetched.spec);
}
