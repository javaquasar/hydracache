use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xtask::memory_ownership::{scan_source_roots, validate_registry_document, OwnershipCandidate};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hydracache-memory-ownership-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp directory");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture_candidates() -> (TempDir, Vec<OwnershipCandidate>) {
    let temp = TempDir::new();
    let source = temp.0.join("fixture/src");
    fs::create_dir_all(&source).expect("source directory");
    fs::write(
        source.join("lib.rs"),
        r#"
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::sync::mpsc::{Receiver, Sender};

type OwnerAlias = HashMap<String, Vec<u8>>;
struct BoundedMapOwner { primary: HashMap<String, Vec<u8>> }
struct HiddenSecondaryMapOwner {
    primary: HashMap<String, Vec<u8>>,
    secondary: HashMap<String, Vec<u8>>,
}
struct ArcCycleState { next: Arc<Mutex<Option<Arc<ArcCycleState>>>> }
struct ChannelState { sender: Sender<Vec<u8>>, receiver: Receiver<Vec<u8>> }
struct DetachedTaskState { task: JoinHandle<()> }
struct AliasStore { entries: OwnerAlias }
struct DatabasePool;
struct ExternalPoolState { pool: DatabasePool }
struct NativeHandle;
struct FfiHandleState { raw: NativeHandle }
define_owner! { struct MacroGeneratedOwner { values: HashMap<String, Vec<u8>> } }
"#,
    )
    .expect("fixture source");
    let candidates = scan_source_roots(&temp.0, &[source]).expect("scan fixture");
    (temp, candidates)
}

fn registry(candidates: &[OwnershipCandidate]) -> String {
    let mut registry = String::from(
        "schema_version = 1\nrelease = \"0.71\"\ndiscovery_scope = \"publishable-workspace-production-rust-v1\"\n\n",
    );
    for candidate in candidates {
        let quote = |value: &str| serde_json::to_string(value).expect("quote");
        let sites = candidate
            .allocation_sites
            .iter()
            .map(|site| quote(site))
            .collect::<Vec<_>>()
            .join(", ");
        registry.push_str(&format!(
            r#"[[owner]]
owner_id = "{}"
subsystem = "fixture"
owning_type = "fixture owner"
source = {}
symbol = {}
allocation_sites = [{}]
keying_dimension = "fixture"
retained_unit = "fixture"
creation_transition = "construct"
terminal_transitions = ["drop", "cancel", "unwind"]
cleanup = "Drop"
count_bound = "scope"
byte_bound = "scope"
age_bound = "scope"
overflow_behavior = "fail"
snapshot_fields = []
security_classification = "aggregate"
focused_tests = ["memory_ownership_071::registry_entry_resolves"]
slow_evidence_ids = []
disposition = "ephemeral"
disposition_reason = "fixture value drops"

"#,
            candidate.candidate_id,
            quote(&candidate.source),
            quote(&candidate.symbol),
            sites
        ));
    }
    registry
}

#[test]
fn scanner_finds_visible_owners_aliases_and_uncertainties() {
    let (_temp, candidates) = fixture_candidates();
    let symbols: Vec<_> = candidates
        .iter()
        .map(|candidate| candidate.symbol.as_str())
        .collect();
    for expected in [
        "BoundedMapOwner",
        "HiddenSecondaryMapOwner",
        "ArcCycleState",
        "ChannelState",
        "DetachedTaskState",
        "AliasStore",
        "ExternalPoolState",
        "FfiHandleState",
    ] {
        assert!(
            symbols.contains(&expected),
            "missing {expected}: {symbols:?}"
        );
    }
    let hidden = candidates
        .iter()
        .find(|candidate| candidate.symbol == "HiddenSecondaryMapOwner")
        .expect("hidden map candidate");
    assert_eq!(hidden.allocation_sites, ["primary", "secondary"]);
    assert!(candidates
        .iter()
        .any(|candidate| candidate.uncertainty.as_deref() == Some("opaque_macro")));
    assert!(candidates.iter().any(|candidate| {
        candidate.symbol == "ExternalPoolState"
            && candidate.uncertainty.as_deref() == Some("external_type")
    }));
}

#[test]
fn registry_entry_resolves() {
    let (_temp, candidates) = fixture_candidates();
    let problems = validate_registry_document(&registry(&candidates), &candidates, "0.71")
        .expect("registry parses");
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn unregistered_secondary_owner_and_renamed_symbol_fail_closed() {
    let (_temp, candidates) = fixture_candidates();
    let mut document = registry(&candidates);
    let target = candidates
        .iter()
        .find(|candidate| candidate.symbol == "HiddenSecondaryMapOwner")
        .expect("target");
    let block_start = document
        .find(&format!("owner_id = \"{}\"", target.candidate_id))
        .expect("owner block");
    let section_start = document[..block_start].rfind("[[owner]]").expect("section");
    let section_end = document[block_start..]
        .find("\n[[owner]]")
        .map(|offset| block_start + offset + 1)
        .unwrap_or(document.len());
    document.replace_range(section_start..section_end, "");
    let problems = validate_registry_document(&document, &candidates, "0.71").expect("validate");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("unreviewed candidate")
            && problem.contains("HiddenSecondaryMapOwner")));

    let renamed = registry(&candidates).replace("BoundedMapOwner", "RenamedOwner");
    let problems = validate_registry_document(&renamed, &candidates, "0.71").expect("validate");
    assert!(problems
        .iter()
        .any(|problem| problem.contains("stale owner")));
}

#[test]
fn expired_symbol_scoped_exemption_fails() {
    let (_temp, candidates) = fixture_candidates();
    let candidate = candidates.first().expect("candidate");
    let document = format!(
        r#"schema_version = 1
release = "0.71"
discovery_scope = "publishable-workspace-production-rust-v1"

[[exemption]]
source = {}
symbol = {}
candidate_kind = {}
reviewer = "fixture-reviewer"
reason = "fixture uncertainty"
expires_after_release = "0.71"
runtime_evidence = ["fixture-runtime"]
terminal_transition_test = "fixture-drop"
"#,
        serde_json::to_string(&candidate.source).unwrap(),
        serde_json::to_string(&candidate.symbol).unwrap(),
        serde_json::to_string(&candidate.candidate_kind).unwrap(),
    );
    let problems = validate_registry_document(&document, std::slice::from_ref(candidate), "0.71")
        .expect("validate exemption");
    assert!(problems.iter().any(|problem| problem.contains("expired")));
}

#[test]
fn checked_in_registry_closes_current_production_inventory() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let problems = xtask::memory_ownership::check(&root, "0.71").expect("ownership check");
    assert!(problems.is_empty(), "{problems:#?}");
}
