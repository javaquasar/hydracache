//! Bounded provisional ledger/source guard. Executable Java reflection and actual-service
//! differential tests are separate mandatory evidence; this is not a full ABI validator.
use hydracache_cluster_testkit::value_plane_capability_075::LIFECYCLE_FEATURES;
use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
pub struct SemanticSources<'a> {
    pub ledger: &'a str,
    pub operations: &'a str,
    pub borrowed: &'a str,
    pub adapter: &'a str,
    pub adapter_tests: &'a str,
    pub signature_tests: &'a str,
    pub java_service: &'a str,
    pub rust_service: &'a str,
}

pub fn run(args: Vec<String>) -> Result<()> {
    if args != ["--release", "0.75"] {
        return Err("usage: imap-semantic-audit --release 0.75".into());
    }
    let problems = check_at_root(&crate::doc_check::find_repo_root()?, "0.75")?;
    if !problems.is_empty() {
        return Err(problems.join("; ").into());
    }
    println!("imap-semantic-audit 0.75: OK (40 local operations, 7 helper families, 8 adapted map expectations; provisional shape guard, not SDK/ABI/production admission)");
    Ok(())
}
pub fn check_at_root(root: &Path, release: &str) -> Result<Vec<String>> {
    if release != "0.75" {
        return Ok(vec!["unsupported semantic ledger release".into()]);
    }
    let read = |path: &str| -> Result<String> {
        Ok(String::from_utf8(
            crate::imap_qualification_075::read_relative(root, path, 512 * 1024)?,
        )?)
    };
    let ledger = read("docs/integrations/imap-compatibility-0.75.md")?;
    let operations = read("docs/testing/imap/0.75/operation-contract.json")?;
    let borrowed = read("docs/integrations/hazelcast_borrowed_suite.json")?;
    let adapter = read("tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapServiceSemanticAdapter.java")?;
    let tests = read("tests/java-imap-benchmark/src/test/java/io/hydracache/imap/semantic/UnifiedIMapServiceSemanticAdapterTest.java")?;
    let signatures = read("tests/java-imap-benchmark/src/test/java/io/hydracache/imap/semantic/IMapSemanticCompatibilityTest.java")?;
    let java = read("tests/java-imap-benchmark/src/main/java/io/hydracache/imap/semantic/UnifiedIMapService.java")?;
    let rust = read("crates/hydracache-cluster-testkit/src/value_plane_service_075.rs")?;
    check_sources(&SemanticSources {
        ledger: &ledger,
        operations: &operations,
        borrowed: &borrowed,
        adapter: &adapter,
        adapter_tests: &tests,
        signature_tests: &signatures,
        java_service: &java,
        rust_service: &rust,
    })
}
pub fn check_sources(s: &SemanticSources<'_>) -> Result<Vec<String>> {
    if [
        s.ledger,
        s.operations,
        s.borrowed,
        s.adapter,
        s.adapter_tests,
        s.signature_tests,
        s.java_service,
        s.rust_service,
    ]
    .iter()
    .any(|source| source.len() > 512 * 1024)
    {
        return Err("semantic audit source exceeds bound".into());
    }
    let contract: Value = serde_json::from_str(s.operations)?;
    let borrowed: Value = serde_json::from_str(s.borrowed)?;
    let operations = contract["operations"]
        .as_array()
        .ok_or("missing operation contract")?;
    let ids = operations
        .iter()
        .map(|row| {
            row["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or("missing operation ID")
        })
        .collect::<std::result::Result<BTreeSet<_>, _>>()?;
    let mut problems = Vec::new();
    if contract["release"] != "0.75"
        || contract["status"] != "provisional"
        || contract["api_extensions"]["production_enabled"] != false
        || ids.len() != operations.len()
    {
        problems.push("operation contract must remain unique and provisional".into());
    }
    for marker in [
        "Ledger version: 1.",
        "Status: provisional.",
        "Expanded production capability: disabled.",
        "Scope: local_reference_only.",
        "not identical language syntax",
        "not a final binary ABI",
        "actual injected service",
        "a9ce2a02ac17f88fcd38869ac698e56e613dc40c",
    ] {
        if !s.ledger.contains(marker) {
            problems.push(format!("missing semantic boundary: {marker}"));
        }
    }
    if rows(s.ledger, "core:")? != ids {
        problems.push("core method ledger differs from operation contract".into());
    }
    if rows(s.ledger, "helper:")? != LIFECYCLE_FEATURES.iter().map(|id| (*id).into()).collect() {
        problems.push("typed helper ledger differs from actual local families".into());
    }
    let adapter = crate::imap_capability_audit::clean(s.adapter)?;
    for marker in [
        "implements MapSemanticAdapter",
        "service.execute(command)",
        "listener.drain()",
        "service.diagnosticSnapshot(tick)",
        "service.signalListenerGap(listener.id())",
    ] {
        if !adapter.contains(marker) {
            problems.push(format!("adapter is missing actual-owner path {marker}"));
        }
    }
    if adapter.contains("Map<BytesValue")
        || adapter.contains("TreeMap<BytesValue")
        || adapter.contains("events.add(")
    {
        problems.push("semantic adapter cannot own a second value/event store".into());
    }
    let signatures = crate::imap_capability_audit::clean(s.signature_tests)?;
    for marker in [
        "getDeclaredMethod",
        "getGenericParameterTypes",
        "getRecordComponents",
        "com.google.protobuf",
        "io.grpc",
        "filtersAndTypedHelpersPreserveBinaryPrefixExactAndNullRules",
        "ttlUpdatePreserveIsRejectedBeforeExpiryAndRetainedIdentityAdmission",
    ] {
        if !signatures.contains(marker) {
            problems.push(format!("missing executable shape/semantic check {marker}"));
        }
    }
    let tests = crate::imap_capability_audit::clean(s.adapter_tests)?;
    for marker in [
        "sharedSeededManifestsUseActualOwner",
        "retainedBorrowedMapExpectationsUseUnifiedOwner",
        "invalidPreserveAndDuplicateBulkNeverChangeStateOrEmit",
        "actualFaultCanariesFailTheOracleAndAllAdapterOwnershipIsReleased",
        "new SemanticOracle()",
        "SeededSemanticScenario.generate",
    ] {
        if !tests.contains(marker) {
            problems.push(format!("missing executable adapter check {marker}"));
        }
    }
    let map_cases = borrowed["rows"]
        .as_array()
        .ok_or("missing borrowed corpus rows")?
        .iter()
        .filter_map(|row| row["id"].as_str())
        .filter(|id| id.starts_with("hz-map-"))
        .collect::<BTreeSet<_>>();
    if borrowed["source"]["commit"] != "a9ce2a02ac17f88fcd38869ac698e56e613dc40c"
        || map_cases.len() != 8
    {
        problems
            .push("retained borrowed map corpus changed without an explicit ledger update".into());
    }
    for id in map_cases {
        if !tests.contains(&format!("\"{id}\"")) {
            problems.push(format!("unmapped borrowed expectation {id}"));
        }
    }
    let java = crate::imap_capability_audit::clean(s.java_service)?;
    let rust = crate::imap_capability_audit::clean(s.rust_service)?;
    for (language, source, markers) in [
        (
            "Java",
            java.as_str(),
            ["BytesValue keyPrefix", "Optional<BytesValue> exactKey"],
        ),
        (
            "Rust",
            rust.as_str(),
            ["pub key_prefix: Vec<u8>", "pub exact_key: Option<Vec<u8>>"],
        ),
    ] {
        for marker in markers {
            if !source.contains(marker) {
                problems.push(format!("{language} listener filter shape drift: {marker}"));
            }
        }
    }
    Ok(problems)
}
fn rows(source: &str, prefix: &str) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    for line in source.lines() {
        if let Some(tail) = line.strip_prefix(&format!("| {prefix}")) {
            let (id, _) = tail
                .split_once(" |")
                .ok_or("malformed semantic ledger row")?;
            if id.is_empty() || !result.insert(id.into()) {
                return Err("duplicate semantic ledger row".into());
            }
        }
    }
    Ok(result)
}
