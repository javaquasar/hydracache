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
    pub rust_entry_views: &'a str,
    pub java_entry_view_tests: &'a str,
    pub rust_entry_view_tests: &'a str,
}

pub fn run(args: Vec<String>) -> Result<()> {
    if args != ["--release", "0.75"] {
        return Err("usage: imap-semantic-audit --release 0.75".into());
    }
    let problems = check_at_root(&crate::doc_check::find_repo_root()?, "0.75")?;
    if !problems.is_empty() {
        return Err(problems.join("; ").into());
    }
    println!("imap-semantic-audit 0.75: OK (40 local operations, 8 helper families, 8 adapted map expectations; provisional shape guard, not SDK/ABI/production admission)");
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
    let rust_entry_views =
        read("crates/hydracache-cluster-testkit/src/value_plane_service_entry_views_075.rs")?;
    let java_entry_view_tests = read("tests/java-imap-benchmark/src/test/java/io/hydracache/imap/semantic/BatchEntryViewsTest.java")?;
    let rust_entry_view_tests =
        read("crates/hydracache-cluster-testkit/tests/value_plane_entry_views_075.rs")?;
    check_sources(&SemanticSources {
        ledger: &ledger,
        operations: &operations,
        borrowed: &borrowed,
        adapter: &adapter,
        adapter_tests: &tests,
        signature_tests: &signatures,
        java_service: &java,
        rust_service: &rust,
        rust_entry_views: &rust_entry_views,
        java_entry_view_tests: &java_entry_view_tests,
        rust_entry_view_tests: &rust_entry_view_tests,
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
        s.rust_entry_views,
        s.java_entry_view_tests,
        s.rust_entry_view_tests,
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
    let helpers = LIFECYCLE_FEATURES
        .iter()
        .map(|id| (*id).into())
        .collect::<BTreeSet<String>>();
    if helpers.len() != 8
        || !helpers.contains("bounded_batch_entry_views")
        || rows(s.ledger, "helper:")? != helpers
    {
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
        "entryViews",
        "UnifiedIMapService.VersionedEntryView.class.getRecordComponents()",
        "IMapLocalHelper.EntryViews.class.getRecordComponents()",
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
    let java_entry_views = java.split_whitespace().collect::<String>();
    let rust_entry_views = crate::imap_capability_audit::clean(s.rust_entry_views)?
        .split_whitespace()
        .collect::<String>();
    for (language, source, markers) in [
        (
            "Java",
            java_entry_views.as_str(),
            [
                "publicList<VersionedEntryView>entryViews(",
                "publicrecordVersionedEntryView",
                "Optional<VersionToken>token",
            ],
        ),
        (
            "Rust",
            rust_entry_views.as_str(),
            [
                "pubfnentry_views(",
                "pubstructVersionedEntryView",
                "pubtoken:Option<VersionToken>",
            ],
        ),
    ] {
        for marker in markers {
            if !source.contains(marker) {
                problems.push(format!(
                    "{language} bounded entry-view shape drift: {marker}"
                ));
            }
        }
    }
    for (language, source, names) in [
        (
            "Java",
            s.java_entry_view_tests,
            [
                "orderedViewsIncludeAbsentDuplicateEntriesAndUsableFullTokens",
                "fullBatchBudgetsRejectBeforeTimeEventsAndOwnersChange",
                "asynchronousViewsFreezeKeysAndRemainReadHelpersWithoutMutationIdentity",
            ],
        ),
        (
            "Rust",
            s.rust_entry_view_tests,
            [
                "batch_entry_views_preserve_order_absence_duplicates_ttl_and_full_tokens",
                "batch_entry_view_bounds_reject_before_clock_expiry_events_or_receipts_change",
                "async_entry_views_are_lazy_bounded_read_helpers_without_retained_receipts",
            ],
        ),
    ] {
        let source = crate::imap_capability_audit::clean(source)?;
        for name in names {
            if !source.contains(name) || !s.ledger.contains(name) {
                problems.push(format!(
                    "missing {language} bounded entry-view evidence {name}"
                ));
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
