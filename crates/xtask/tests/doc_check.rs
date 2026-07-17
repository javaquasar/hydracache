//! Tests for the release-manifest consistency checker (`xtask doc-check`).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::doc_check;

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a throwaway repo root under the system temp dir with the given
/// `releases.toml` body and (optionally) referenced plan files.
fn scratch_root(manifest: &str, plan_files: &[&str]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hydracache_doc_check_{}_{nanos}_{counter}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("docs/plans")).unwrap();
    fs::write(root.join("docs/plans/releases.toml"), manifest).unwrap();
    for file in plan_files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "# plan\n").unwrap();
    }
    root
}

fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}

fn write_redis_test_source(root: &Path, source: &str, tests: &[&str], gated: bool) {
    let path = root.join(source);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut body = String::new();
    for test in tests {
        if gated {
            body.push_str(&format!(
                "#[test]\nfn {test}() {{ if !skip_unless_redis_resp_multinode_e2e(\"{test}\") {{ return; }} }}\n"
            ));
        } else {
            body.push_str(&format!("#[test]\nfn {test}() {{}}\n"));
        }
    }
    fs::write(path, body).unwrap();
}

fn valid_redis_compat_manifest() -> &'static str {
    r#"{
  "version": 1,
  "surface": "hydracache-redis-resp-edge",
  "supported_resp": "RESP2+RESP3",
  "redis_oracle": {
    "images": ["redis:6.2.14", "redis:7.2.5"],
    "normalization": "exact for supported commands"
  },
  "test_layers": [
    {"id":"contract","kind":"contract_suite","description":"backend contract","source":"crates/redis_tests.rs","tests":["get_matches_real_redis"]},
    {"id":"goldens","kind":"characterization","description":"wire goldens","source":"crates/redis_tests.rs","tests":["info_returns_minimal_metadata","hc_stats_is_hydracache_only"]},
    {"id":"flips","kind":"flip_sentinel","description":"deployment flips","source":"crates/hydracache-server/tests/redis_resp_multinode.rs","tests":["multinode_resp_facade_documents_node_local_state","cross_node_mget_del_exists_are_node_local","cross_node_mset_is_node_local","multinode_resp_lock_subset_is_single_endpoint_only","cross_node_lock_release_is_node_local","cross_node_lock_extend_is_node_local"]}
  ],
  "commands": [
    {"name":"GET","status":"supported","kind":"redis_subset","route":"translator","case_ids":["get"],"oracle":"exact","tests":["get_matches_real_redis"]},
    {"name":"INFO","status":"supported_with_caveat","kind":"health_probe","route":"listener_metrics","case_ids":["info"],"oracle":"normalized_metadata","tests":["info_returns_minimal_metadata"]},
    {"name":"HC.STATS","status":"hydracache_extension","kind":"hydracache_extension","route":"translator","case_ids":["hc-stats"],"oracle":"hydracache_only","tests":["hc_stats_is_hydracache_only"]},
    {"name":"Cross-endpoint RESP key visibility","status":"unsupported","kind":"deployment_scope","route":"deployment_scope","case_ids":["cross-endpoint-get"],"oracle":"documented_divergence","debt_id":"resp-cross-endpoint-key-visibility","current_claim":"node local","target_claim":"distributed","tests":["multinode_resp_facade_documents_node_local_state","cross_node_mget_del_exists_are_node_local","cross_node_mset_is_node_local"]},
    {"name":"Multi-endpoint Redis lock mutual exclusion","status":"unsupported","kind":"deployment_scope","route":"deployment_scope","case_ids":["cross-endpoint-set-nx"],"oracle":"documented_divergence","debt_id":"resp-cross-endpoint-lock-safety","current_claim":"single endpoint","target_claim":"distributed locks","tests":["multinode_resp_lock_subset_is_single_endpoint_only","cross_node_lock_release_is_node_local","cross_node_lock_extend_is_node_local"]}
  ]
}"#
}

fn write_valid_redis_test_catalog(root: &Path) {
    write_redis_test_source(
        root,
        "crates/redis_tests.rs",
        &[
            "get_matches_real_redis",
            "info_returns_minimal_metadata",
            "hc_stats_is_hydracache_only",
        ],
        false,
    );
    write_redis_test_source(
        root,
        "crates/hydracache-server/tests/redis_resp_multinode.rs",
        &[
            "multinode_resp_facade_documents_node_local_state",
            "cross_node_mget_del_exists_are_node_local",
            "cross_node_mset_is_node_local",
            "multinode_resp_lock_subset_is_single_endpoint_only",
            "cross_node_lock_release_is_node_local",
            "cross_node_lock_extend_is_node_local",
        ],
        true,
    );
}

#[test]
fn consistent_manifest_has_no_problems() {
    let manifest = r#"
[[release]]
version = "0.37.0"
file = "docs/plans/A.md"
status = "shipped"
depends_on = []

[[release]]
version = "0.38.0"
file = "docs/plans/B.md"
status = "planned"
depends_on = ["0.37.0"]

[[release]]
version = "TBD"
file = "docs/plans/DRAFT.md"
status = "draft"
depends_on = ["0.38.0"]
"#;
    let root = scratch_root(
        manifest,
        &["docs/plans/A.md", "docs/plans/B.md", "docs/plans/DRAFT.md"],
    );
    fs::create_dir_all(root.join("docs/releases")).unwrap();
    fs::write(
        root.join("docs/releases/0.37.0.md"),
        "# HydraCache 0.37.0\n",
    )
    .unwrap();
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    assert!(
        problems.is_empty(),
        "expected no problems, got: {problems:?}"
    );
}

#[test]
fn release_work_items_match_plan_headings_and_index_marker() {
    let manifest = r#"
[[release]]
version = "0.64.0"
file = "docs/plans/V0_64.md"
status = "planned"
work_items = ["W1", "W5a", "W6"]
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_64.md"]);
    fs::write(
        root.join("docs/plans/V0_64.md"),
        "# plan\n\n## W1. First\n\n### W5a. Transport\n\n## W6. Gates\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/plans/INDEX.md"),
        "<!-- release-work-items:0.64.0=W1,W5a,W6 -->\n",
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    assert!(
        problems.is_empty(),
        "expected no problems, got: {problems:?}"
    );
}

#[test]
fn detects_release_work_item_drift_across_manifest_plan_and_index() {
    let manifest = r#"
[[release]]
version = "0.64.0"
file = "docs/plans/V0_64.md"
status = "planned"
work_items = ["W1", "W2", "W2", "Wbad"]
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_64.md"]);
    fs::write(
        root.join("docs/plans/V0_64.md"),
        "# plan\n\n## W1. First\n\n## W3. Undeclared\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/plans/INDEX.md"),
        "<!-- release-work-items:0.64.0=W1 -->\n",
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    let joined = problems.join("\n");
    assert!(
        joined.contains("manifest work item 'W2' has no matching plan heading"),
        "missing declared-heading drift: {joined}"
    );
    assert!(
        joined.contains("plan work item 'W3' is missing from releases.toml work_items"),
        "missing undeclared-heading drift: {joined}"
    );
    assert!(
        joined.contains("missing release work-item marker"),
        "missing index-marker drift: {joined}"
    );
    assert!(
        joined.contains("duplicate work item 'W2'"),
        "missing duplicate work-item validation: {joined}"
    );
    assert!(
        joined.contains("invalid work item 'Wbad'"),
        "missing invalid work-item validation: {joined}"
    );
}

#[test]
fn detects_shipped_release_without_release_notes() {
    let manifest = r#"
[[release]]
version = "0.37.0"
file = "docs/plans/A.md"
status = "shipped"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/A.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("shipped release '0.37.0' is missing docs/releases/0.37.0.md"),
        "missing release-note check: {joined}"
    );
}

#[test]
fn detects_duplicate_version_missing_file_bad_status_and_dangling_dep() {
    let manifest = r#"
[[release]]
version = "0.40.0"
file = "docs/plans/A.md"
status = "planned"
depends_on = ["9.9.9"]

[[release]]
version = "0.40.0"
file = "docs/plans/missing.md"
status = "weird"
depends_on = []

[[release]]
version = "TBD"
file = "docs/plans/A.md"
status = "planned"
depends_on = []
"#;
    // Only A.md exists; missing.md does not.
    let root = scratch_root(manifest, &["docs/plans/A.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("duplicate version '0.40.0'"),
        "missing dup check: {joined}"
    );
    assert!(
        joined.contains("file does not exist"),
        "missing file-existence check: {joined}"
    );
    assert!(
        joined.contains("invalid status 'weird'"),
        "missing status check: {joined}"
    );
    assert!(
        joined.contains("depends_on '9.9.9'"),
        "missing dangling-dep check: {joined}"
    );
    assert!(
        joined.contains("version 'TBD' is only allowed"),
        "missing TBD-on-non-draft check: {joined}"
    );
}

#[test]
fn detects_shipped_043_without_networked_control_plane_sentinel() {
    let manifest = r#"
[[release]]
version = "0.43.0"
file = "docs/plans/V0_43.md"
status = "shipped"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_43.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("shipped 0.43.0 must set networked_control_plane = true"),
        "missing 0.43 sentinel check: {joined}"
    );
}

#[test]
fn detects_shipped_release_with_false_networked_control_plane_sentinel() {
    let manifest = r#"
[[release]]
version = "0.43.0"
file = "docs/plans/V0_43.md"
status = "shipped"
networked_control_plane = false
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_43.md"]);
    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("shipped release cannot set networked_control_plane = false"),
        "missing false-sentinel check: {joined}"
    );
}

#[test]
fn detects_dangling_in_prose_plan_links() {
    let manifest = r#"
[[release]]
version = "0.50.0"
file = "docs/plans/V0_50_EXISTING_PLAN.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_50_EXISTING_PLAN.md"]);
    fs::write(
        root.join("docs/plans/V0_50_EXISTING_PLAN.md"),
        "See `V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md` and `V0_99_MISSING_PLAN.md`.\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/plans/V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md"),
        "# existing plan\n",
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("references missing plan 'V0_99_MISSING_PLAN.md'"),
        "missing in-prose plan-link check: {joined}"
    );
    assert!(
        !joined.contains("V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md"),
        "existing in-prose plan link should not fail: {joined}"
    );
}

#[test]
fn detects_duplicate_adr_numbers_old_scheme_and_missing_index_entries() {
    let manifest = r#"
[[release]]
version = "0.50.0"
file = "docs/plans/V0_50_EXISTING_PLAN.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_50_EXISTING_PLAN.md"]);
    let adr_dir = root.join("docs/adr");
    fs::create_dir_all(&adr_dir).unwrap();
    fs::write(
        adr_dir.join("README.md"),
        "[ADR-0001](0001-first.md)\n[ADR-0002](0002-listed.md)\n",
    )
    .unwrap();
    fs::write(adr_dir.join("0001-first.md"), "# ADR-0001: First\n").unwrap();
    fs::write(adr_dir.join("0001-duplicate.md"), "# ADR-0001: Duplicate\n").unwrap();
    fs::write(adr_dir.join("ADR-0002-old-scheme.md"), "# ADR-0002: Old\n").unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("duplicate ADR number 0001"),
        "missing duplicate ADR number check: {joined}"
    );
    assert!(
        joined.contains("ADR filename must use NNNN-title.md"),
        "missing ADR filename scheme check: {joined}"
    );
    assert!(
        joined.contains("missing ADR index entry for 0001-duplicate.md"),
        "missing ADR index coverage check: {joined}"
    );
}

#[test]
fn accepts_valid_redis_compat_conformance_manifest() {
    let manifest = r#"
[[release]]
version = "0.63.0"
file = "docs/plans/V0_63.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_63.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis-compat.md"),
        redis_compat_docs_with_examples(),
    )
    .unwrap();
    fs::write(
        integration_dir.join("redis_compat_conformance.json"),
        valid_redis_compat_manifest(),
    )
    .unwrap();
    write_valid_redis_test_catalog(&root);

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    assert!(
        problems.is_empty(),
        "expected no problems, got: {problems:?}"
    );
}

#[test]
fn accepts_real_proptest_macro_tests_as_manifest_references() {
    let manifest = r#"
[[release]]
version = "0.63.0"
file = "docs/plans/V0_63.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_63.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis-compat.md"),
        redis_compat_docs_with_examples(),
    )
    .unwrap();
    fs::write(
        integration_dir.join("redis_compat_conformance.json"),
        valid_redis_compat_manifest(),
    )
    .unwrap();
    write_valid_redis_test_catalog(&root);
    fs::write(
        root.join("crates/redis_tests.rs"),
        r#"
#[test]
fn get_matches_real_redis() {}
#[test]
fn hc_stats_is_hydracache_only() {}
proptest! {
    #[test]
    fn info_returns_minimal_metadata(input in 0_u8..=255) { let _ = input; }
}
"#,
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    assert!(
        problems.is_empty(),
        "expected a proptest-generated #[test] to resolve, got: {problems:?}"
    );
}

#[test]
fn rejects_redis_compat_deployment_scope_without_multinode_sentinel() {
    let manifest = r#"
[[release]]
version = "0.63.0"
file = "docs/plans/V0_63.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_63.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis-compat.md"),
        redis_compat_docs_with_examples(),
    )
    .unwrap();
    fs::write(
        integration_dir.join("redis_compat_conformance.json"),
        r#"{
  "version": 1,
  "surface": "hydracache-redis-resp-edge",
  "supported_resp": "RESP2+RESP3",
  "redis_oracle": {
    "images": ["redis:6.2.14", "redis:7.2.5"],
    "normalization": "exact for supported commands"
  },
  "commands": [
    {
      "name": "Cross-endpoint RESP key visibility",
      "status": "unsupported",
      "kind": "deployment_scope",
      "route": "deployment_scope",
      "case_ids": ["cross-endpoint-get"],
      "oracle": "documented_divergence",
      "debt_id": "resp-cross-endpoint-key-visibility",
      "current_claim": "node local",
      "target_claim": "distributed",
      "tests": ["missing_multinode_sentinel"]
    }
  ]
}"#,
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("deployment_scope test 'missing_multinode_sentinel' must be a real #[test] in crates/hydracache-server/tests/redis_resp_multinode.rs"),
        "missing deployment_scope sentinel implementation check: {joined}"
    );
}

#[test]
fn rejects_commented_or_ungated_redis_multinode_sentinels() {
    let manifest = r#"
[[release]]
version = "0.65.0"
file = "docs/plans/V0_65.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_65.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis-compat.md"),
        redis_compat_docs_with_examples(),
    )
    .unwrap();
    fs::write(
        integration_dir.join("redis_compat_conformance.json"),
        valid_redis_compat_manifest(),
    )
    .unwrap();
    write_valid_redis_test_catalog(&root);
    fs::write(
        root.join("crates/hydracache-server/tests/redis_resp_multinode.rs"),
        r#"
// #[test]
// fn multinode_resp_facade_documents_node_local_state() {}
#[test]
fn cross_node_mget_del_exists_are_node_local() {}
#[test]
fn cross_node_mset_is_node_local() { skip_unless_redis_resp_multinode_e2e("cross_node_mset_is_node_local"); }
#[test]
fn multinode_resp_lock_subset_is_single_endpoint_only() { skip_unless_redis_resp_multinode_e2e("multinode_resp_lock_subset_is_single_endpoint_only"); }
#[test]
fn cross_node_lock_release_is_node_local() { skip_unless_redis_resp_multinode_e2e("cross_node_lock_release_is_node_local"); }
#[test]
fn cross_node_lock_extend_is_node_local() { skip_unless_redis_resp_multinode_e2e("cross_node_lock_extend_is_node_local"); }
"#,
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    let joined = problems.join("\n");
    assert!(
        joined.contains("deployment_scope test 'multinode_resp_facade_documents_node_local_state' must be a real #[test]"),
        "commented function was accepted as a sentinel: {joined}"
    );
    assert!(
        joined.contains("deployment_scope test 'cross_node_mget_del_exists_are_node_local' must call skip_unless_redis_resp_multinode_e2e"),
        "ungated function was accepted as a sentinel: {joined}"
    );
}

#[test]
fn rejects_dangling_tests_missing_debt_ids_and_duplicate_case_ids() {
    let manifest = r#"
[[release]]
version = "0.65.0"
file = "docs/plans/V0_65.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_65.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis-compat.md"),
        redis_compat_docs_with_examples(),
    )
    .unwrap();
    let broken = valid_redis_compat_manifest()
        .replacen("\"case_ids\":[\"info\"]", "\"case_ids\":[\"get\"]", 1)
        .replace(
            "resp-cross-endpoint-key-visibility",
            "renamed-cross-endpoint-key-visibility",
        )
        .replace("get_matches_real_redis", "dangling_get_test");
    fs::write(
        integration_dir.join("redis_compat_conformance.json"),
        broken,
    )
    .unwrap();
    write_valid_redis_test_catalog(&root);

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);
    let joined = problems.join("\n");
    assert!(
        joined.contains("duplicate global case id 'get'"),
        "{joined}"
    );
    assert!(
        joined.contains("test 'dangling_get_test' does not resolve to a real Rust #[test]"),
        "{joined}"
    );
    assert!(
        joined.contains("missing required stable debt_id 'resp-cross-endpoint-key-visibility'"),
        "{joined}"
    );
}

#[test]
fn rejects_redis_compat_docs_examples_without_gate_labels() {
    let manifest = r#"
[[release]]
version = "0.63.0"
file = "docs/plans/V0_63.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_63.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis-compat.md"),
        "# Redis RESP\n\n## Executable Examples\n\n### redis-cli\n\n```sh\nredis-cli PING\n```\n",
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("example section '### redis-cli' must name Gate: `redis_clients`"),
        "missing redis-cli gate-label check: {joined}"
    );
    assert!(
        joined.contains("missing executable example section '### Rust (redis-rs)'"),
        "missing required language example check: {joined}"
    );
}

fn redis_compat_docs_with_examples() -> &'static str {
    r#"# Redis RESP

## Executable Examples

### redis-cli

Gate: `redis_clients`

```sh
redis-cli PING
```

### Rust (redis-rs)

Gate: `redis_clients`

```rust
let _ = redis::cmd("PING");
```

### Python (redis-py)

Gate: `redis_clients`

```python
print("PING")
```

### Node (node-redis)

Gate: `redis_clients`

```javascript
console.log("PING");
```

### Go (go-redis)

Gate: `redis_clients`

```go
fmt.Println("PING")
```

### JVM (Jedis)

Gate: `redis_clients`

```java
System.out.println("PING");
```
"#
}

#[test]
fn rejects_redis_compat_manifest_drift() {
    let manifest = r#"
[[release]]
version = "0.63.0"
file = "docs/plans/V0_63.md"
status = "planned"
depends_on = []
"#;
    let root = scratch_root(manifest, &["docs/plans/V0_63.md"]);
    let integration_dir = root.join("docs/integrations");
    fs::create_dir_all(&integration_dir).unwrap();
    fs::write(
        integration_dir.join("redis_compat_conformance.json"),
        r#"{
  "version": 2,
  "surface": "wrong",
  "supported_resp": "RESP3",
  "redis_oracle": {
    "images": ["redis:latest"],
    "normalization": ""
  },
  "commands": [
    {
      "name": "GET",
      "status": "supported",
      "kind": "redis_subset",
      "oracle": "candidate",
      "tests": []
    },
    {
      "name": "GET",
      "status": "mystery",
      "kind": "",
      "oracle": "unknown",
      "tests": ["bad test name"]
    }
  ]
}"#,
    )
    .unwrap();

    let problems = doc_check::check(&root).unwrap();
    cleanup(&root);

    let joined = problems.join("\n");
    assert!(
        joined.contains("redis-compat.md: file does not exist"),
        "missing docs file check: {joined}"
    );
    assert!(
        joined.contains("unsupported manifest version 2"),
        "missing version check: {joined}"
    );
    assert!(
        joined.contains("unexpected surface 'wrong'"),
        "missing surface check: {joined}"
    );
    assert!(
        joined.contains("supported_resp must be RESP2+RESP3"),
        "missing RESP2+RESP3 check: {joined}"
    );
    assert!(
        joined.contains("redis_oracle image 'redis:latest' must be pinned"),
        "missing pinned-image check: {joined}"
    );
    assert!(
        joined.contains("redis_oracle.normalization must not be empty"),
        "missing normalization check: {joined}"
    );
    assert!(
        joined.contains("status 'supported' requires at least one covering test"),
        "missing supported-test check: {joined}"
    );
    assert!(
        joined.contains("supported Redis command cannot use oracle 'candidate'"),
        "missing supported-oracle check: {joined}"
    );
    assert!(
        joined.contains("duplicate command name"),
        "missing duplicate command check: {joined}"
    );
    assert!(
        joined.contains("invalid status 'mystery'"),
        "missing status check: {joined}"
    );
    assert!(
        joined.contains("invalid oracle 'unknown'"),
        "missing oracle check: {joined}"
    );
    assert!(
        joined.contains("kind must not be empty"),
        "missing kind check: {joined}"
    );
    assert!(
        joined.contains("test names must be non-empty identifiers"),
        "missing test-name check: {joined}"
    );
}
