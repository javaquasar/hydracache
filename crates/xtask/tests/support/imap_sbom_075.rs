use super::*;
fn component(id: &str) -> Component {
    Component {
        id: id.into(),
        ecosystem: "maven".into(),
        name: "fixture".into(),
        version: "1".into(),
        source: "fixture".into(),
        source_checksum: "a".repeat(64),
        upstream_source: "unknown".into(),
        upstream_origin: "unknown".into(),
        licenses: vec![],
        license_origin: "unknown".into(),
    }
}
#[test]
fn component_merge_refuses_substitution_and_excessive_work() {
    let mut components = BTreeMap::new();
    insert(&mut components, component("fixture")).unwrap();
    let mut substituted = component("fixture");
    substituted.source = "substituted".into();
    assert!(insert(&mut components, substituted)
        .unwrap_err()
        .to_string()
        .contains("substituted"));
    components = (0..8192)
        .map(|i| (i.to_string(), component(&i.to_string())))
        .collect();
    assert!(insert(&mut components, component("overflow"))
        .unwrap_err()
        .to_string()
        .contains("work bound"));
}
fn node() -> Value {
    json!({"groupId":"a","artifactId":"b","type":"jar","version":"1","scope":"compile"})
}
#[test]
fn maven_tree_rejects_missing_material_unknown_scope_and_depth_overflow() {
    let known = BTreeMap::from([("maven:a:b:jar::1".into(), component("maven:a:b:jar::1"))]);
    assert!(tree_edges(&node(), "fixture", &BTreeMap::new())
        .unwrap_err()
        .to_string()
        .contains("root/material missing"));
    let mut tree = node();
    let mut child = node();
    child["artifactId"] = json!("foreign");
    tree["children"] = json!([child]);
    assert!(tree_edges(&tree, "fixture", &known)
        .unwrap_err()
        .to_string()
        .contains("dependency missing"));
    let mut child = node();
    child["scope"] = json!("future");
    tree["children"] = json!([child]);
    assert!(tree_edges(&tree, "fixture", &known)
        .unwrap_err()
        .to_string()
        .contains("unknown dependency scope"));
    tree = node();
    for _ in 0..130 {
        let mut parent = node();
        parent["children"] = json!([tree]);
        tree = parent;
    }
    assert!(tree_edges(&tree, "fixture", &known)
        .unwrap_err()
        .to_string()
        .contains("work/depth bound"));
}
