//! Tests for the release-manifest consistency checker (`xtask doc-check`).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::doc_check;

/// Create a throwaway repo root under the system temp dir with the given
/// `releases.toml` body and (optionally) referenced plan files.
fn scratch_root(manifest: &str, plan_files: &[&str]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("hydracache_doc_check_{nanos}"));
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
      "name": "GET",
      "status": "supported",
      "kind": "redis_subset",
      "oracle": "exact",
      "tests": ["get_matches_real_redis"]
    },
    {
      "name": "HC.STATS",
      "status": "hydracache_extension",
      "kind": "hydracache_extension",
      "oracle": "hydracache_only",
      "tests": ["hc_stats_is_hydracache_only"]
    }
  ]
}"#,
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
