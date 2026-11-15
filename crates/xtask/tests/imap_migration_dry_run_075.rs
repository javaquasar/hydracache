use hydracache_cluster_testkit::value_plane_collection_075::CollectionBounds;
use hydracache_cluster_testkit::value_plane_model_075::MutationIdentity;
use hydracache_cluster_testkit::value_plane_service_075::{ServiceCommand, UnifiedIMapService};
use serde_json::{json, Value};
use std::path::Path;
use xtask::imap_migration_dry_run::{chunk, digest, load_corpus, DryRun, PostCutover};

const CORPUS: &[u8] = include_bytes!("../../../docs/testing/imap/0.75/migration-fixtures.json");
fn live(generation: u64) -> UnifiedIMapService {
    UnifiedIMapService::new(CollectionBounds::default(), generation).unwrap()
}

#[test]
fn migration_dry_run_checks_actual_staging_ttl_digest_and_leaves_live_untouched() {
    let corpus = load_corpus(CORPUS).unwrap();
    let fixture = corpus.fixtures[0].clone();
    let live = live(fixture.generation);
    let mut write = ServiceCommand::point("set", b"live");
    write.value = Some(b"authoritative".to_vec());
    write.identity = Some(MutationIdentity::new("live", 1));
    live.execute(write).unwrap();
    let before = live.diagnostic_snapshot(0).unwrap();
    let mut dry = DryRun::new(fixture.clone(), corpus.limits, live.clone()).unwrap();
    assert!(dry.finish().is_err());
    let first = chunk(&fixture, 0, fixture.entries[..2].to_vec()).unwrap();
    assert!(!dry.accept(&first).unwrap().duplicate);
    let usage = dry.staging().resource_usage();
    assert!(dry.accept(&first).unwrap().duplicate);
    assert_eq!(usage, dry.staging().resource_usage());
    dry.accept(&chunk(&fixture, 1, fixture.entries[2..].to_vec()).unwrap())
        .unwrap();
    let receipt = dry.finish().unwrap();
    assert_eq!(receipt["entries"], 4);
    assert_eq!(receipt["chunks"], 2);
    assert_eq!(receipt["live_unchanged"], true);
    assert_eq!(receipt["source_digest"], receipt["staging_digest"]);
    let ttl = dry
        .staging()
        .diagnostic_snapshot(0)
        .unwrap()
        .iter()
        .filter_map(|entry| entry.remaining_ttl)
        .collect::<Vec<_>>();
    assert_eq!(ttl.len(), 2);
    assert!(ttl.contains(&7));
    assert!(ttl.contains(&11));
    assert_eq!(before, live.diagnostic_snapshot(0).unwrap());
    dry.close().unwrap();
    dry.close().unwrap();
    assert_eq!(dry.staging().resource_usage().values, 0);
    assert_eq!(dry.staging().resource_usage().outcomes, 0);
    assert!(dry.finish().is_err());
    assert!(dry.accept(&first).is_err());
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let result = xtask::imap_migration_dry_run::check_at_root(root, "0.75").unwrap();
    assert_eq!(result["fixtures"].as_array().unwrap().len(), 2);
    assert_eq!(result["production_import"], false);
    assert!(xtask::imap_migration_dry_run::check_at_root(root, "0.74").is_err());
}

#[test]
fn migration_chunks_refuse_scope_order_checksum_duplicates_and_budgets_before_writes() {
    let corpus = load_corpus(CORPUS).unwrap();
    let fixture = corpus.fixtures[0].clone();
    let first = chunk(&fixture, 0, fixture.entries[..2].to_vec()).unwrap();
    let mut dry = DryRun::new(fixture.clone(), corpus.limits.clone(), live(7)).unwrap();
    let mut cases = Vec::new();
    let mut wrong = first.clone();
    wrong.namespace.push('x');
    cases.push(wrong);
    let mut wrong = first.clone();
    wrong.codec_id = "java_serialization".into();
    cases.push(wrong);
    let mut wrong = first.clone();
    wrong.tenant.push('x');
    cases.push(wrong);
    let mut wrong = first.clone();
    wrong.generation += 1;
    cases.push(wrong);
    let mut wrong = first.clone();
    wrong.source_snapshot.push('x');
    cases.push(wrong);
    let mut wrong = first.clone();
    wrong.digest = "0".repeat(64);
    cases.push(wrong);
    cases.push(chunk(&fixture, 1, fixture.entries[..2].to_vec()).unwrap());
    cases.push(chunk(&fixture, 0, vec![]).unwrap());
    cases.push(chunk(&fixture, 0, fixture.entries[..3].to_vec()).unwrap());
    cases.push(
        chunk(
            &fixture,
            0,
            vec![fixture.entries[0].clone(), fixture.entries[0].clone()],
        )
        .unwrap(),
    );
    for refused in cases {
        assert!(dry.accept(&refused).is_err());
        assert!(dry.staging().diagnostic_snapshot(0).unwrap().is_empty());
    }
    dry.accept(&first).unwrap();
    let mut changed = fixture.entries[..2].to_vec();
    changed[0].value_hex = "01".into();
    assert!(dry.accept(&chunk(&fixture, 0, changed).unwrap()).is_err());
    let mut tiny = corpus.limits;
    tiny.max_chunk_bytes = 1;
    let mut limited = DryRun::new(fixture, tiny, live(7)).unwrap();
    assert!(limited.accept(&first).is_err());
    assert_eq!(limited.staging().resource_usage().values, 0);
}

#[test]
fn migration_finish_detects_a_real_staging_defect_and_fixture_decoder_is_strict() {
    let corpus = load_corpus(CORPUS).unwrap();
    let fixture = corpus.fixtures[0].clone();
    let mut dry = DryRun::new(fixture.clone(), corpus.limits.clone(), live(7)).unwrap();
    for (index, entries) in fixture.entries.chunks(2).enumerate() {
        dry.accept(&chunk(&fixture, index, entries.to_vec()).unwrap())
            .unwrap();
    }
    let key = dry.staging().diagnostic_snapshot(0).unwrap()[0].key.clone();
    let mut defect = ServiceCommand::point("set", &key);
    defect.value = Some(vec![99]);
    defect.identity = Some(MutationIdentity::new("actual-defect", 1));
    dry.staging().execute(defect).unwrap();
    assert!(dry.finish().is_err());
    let original: Value = serde_json::from_slice(CORPUS).unwrap();
    let mutations: &[fn(&mut Value)] = &[
        |v| v["production_import"] = json!(true),
        |v| v["mode"] = json!("online_cdc"),
        |v| v["fixture_version"] = json!(2),
        |v| v["limits"]["max_entries"] = json!(0),
        |v| v["limits"]["max_total_bytes"] = json!(1),
        |v| v["fixtures"][0]["expected_digest"] = json!("0".repeat(64)),
        |v| v["fixtures"][0]["codec_id"] = json!("unknown"),
        |v| v["fixtures"][0]["expected_count"] = json!(99),
        |v| v["fixtures"][0]["entries"][0]["key_hex"] = json!("f"),
        |v| v["application_codec_vectors"][1]["encoded_hex"] = json!("00"),
        |v| v["unexpected"] = json!(true),
    ];
    for mutate in mutations {
        let mut broken = original.clone();
        mutate(&mut broken);
        assert!(load_corpus(&serde_json::to_vec(&broken).unwrap()).is_err());
    }
    assert!(load_corpus(b"{").is_err());
    assert!(load_corpus(&vec![b' '; 1024 * 1024 + 1]).is_err());
    let mut wrong_ttl = fixture.clone();
    wrong_ttl.entries[0].ttl_remaining = Some(0);
    wrong_ttl.expected_digest = digest(&wrong_ttl, &wrong_ttl.entries, None).unwrap();
    assert!(DryRun::new(wrong_ttl, corpus.limits, live(7)).is_err());
}

#[test]
fn migration_cli_refuses_bad_arguments_and_never_overwrites_a_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let fixtures = temp.path().join("fixtures.json");
    std::fs::write(&fixtures, CORPUS).unwrap();
    let output = temp.path().join("receipt.json");
    let args = vec![
        "--release".into(),
        "0.75".into(),
        "--fixtures".into(),
        fixtures.to_string_lossy().into_owned(),
        "--output".into(),
        output.to_string_lossy().into_owned(),
    ];
    xtask::imap_migration_dry_run::run(args.clone()).unwrap();
    let original = std::fs::read(&output).unwrap();
    assert!(xtask::imap_migration_dry_run::run(args).is_err());
    assert_eq!(original, std::fs::read(output).unwrap());
    for args in [
        vec![],
        vec!["--release".into()],
        vec!["--release".into(), "0.74".into()],
        vec!["--unknown".into(), "x".into()],
        vec![
            "--release".into(),
            "0.75".into(),
            "--release".into(),
            "0.75".into(),
        ],
    ] {
        assert!(xtask::imap_migration_dry_run::run(args).is_err());
    }
}

fn ready_fixture() -> DryRun {
    let corpus = load_corpus(CORPUS).unwrap();
    let fixture = corpus.fixtures[0].clone();
    let mut dry = DryRun::new(fixture.clone(), corpus.limits, live(fixture.generation)).unwrap();
    assert!(dry.shadow_reads(7).is_err());
    for (index, entries) in fixture.entries.chunks(2).enumerate() {
        dry.accept(&chunk(&fixture, index, entries.to_vec()).unwrap())
            .unwrap();
    }
    dry
}

#[test]
fn rehearsal_compares_shadow_reads_at_exact_expiry_and_preserves_the_frozen_source() {
    let mut before = ready_fixture();
    let (before_expiry, digest_before) = before.shadow_reads(6).unwrap();
    assert_eq!(before_expiry, 4);
    let (after_expiry, digest_after) = before.shadow_reads(7).unwrap();
    assert_eq!(after_expiry, 3);
    assert_ne!(digest_before, digest_after);
    let read_only = before.rehearse(7, PostCutover::ReadOnly).unwrap();
    assert_eq!(read_only.shadow_reads, 4);
    assert_eq!(read_only.visible_entries, 3);
    assert_eq!(read_only.rollback_digest, Some(digest_after.clone()));
    assert!(!read_only.destination_write_applied);
    assert!(before.shadow_reads(7).is_err());
    assert_eq!(before.staging().resource_usage().values, 0);

    let mut discard = ready_fixture();
    let receipt = discard.rehearse(7, PostCutover::DiscardWrites).unwrap();
    assert!(receipt.destination_write_applied);
    assert_eq!(receipt.pre_cutover_digest, digest_after);
    assert_eq!(receipt.rollback_digest, Some(receipt.pre_cutover_digest));
    assert_eq!(discard.staging().resource_usage().outcomes, 0);

    let mut refuse = ready_fixture();
    let receipt = refuse.rehearse(7, PostCutover::RefuseWrites).unwrap();
    assert!(receipt.destination_write_applied);
    assert_eq!(receipt.rollback_digest, None);
    assert_eq!(refuse.staging().diagnostic_snapshot(7).unwrap().len(), 2);
    refuse.close().unwrap();
    assert!(ready_fixture().shadow_reads(u64::MAX).is_err());
}

#[test]
fn shadow_read_rejects_value_ttl_expired_key_and_cardinality_divergence() {
    use hydracache_cluster_testkit::canonical_key_075::{
        ReferenceCanonicalKey, ReferenceKeyBounds,
    };
    use hydracache_cluster_testkit::value_plane_service_075::ServiceTtl;

    for defect in [
        "different_value",
        "different_ttl",
        "expired_key",
        "extra_key",
    ] {
        let mut dry = ready_fixture();
        let fixture = load_corpus(CORPUS).unwrap().fixtures.remove(0);
        let item = match defect {
            "expired_key" => fixture
                .entries
                .iter()
                .find(|entry| entry.ttl_remaining == Some(7))
                .unwrap(),
            _ => fixture
                .entries
                .iter()
                .find(|entry| entry.ttl_remaining.is_none())
                .unwrap(),
        };
        let bytes = if defect == "extra_key" {
            vec![100, 200]
        } else {
            item.key_hex
                .as_bytes()
                .chunks_exact(2)
                .map(|digits| u8::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap())
                .collect()
        };
        let key = ReferenceCanonicalKey::new(
            &fixture.tenant,
            &fixture.namespace,
            fixture.generation,
            bytes,
        )
        .encode(ReferenceKeyBounds::default())
        .unwrap();
        let mut write = ServiceCommand::point("set", &key);
        write.identity = Some(MutationIdentity::new("rehearsal-fault", 1));
        write.value = Some(if defect == "different_ttl" {
            item.value_hex
                .as_bytes()
                .chunks_exact(2)
                .map(|digits| u8::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap())
                .collect()
        } else {
            vec![100, 200]
        });
        if defect == "different_ttl" {
            write.ttl = ServiceTtl::ExpireAfter(100);
        }
        dry.staging().execute(write).unwrap();
        let error = dry
            .rehearse(7, PostCutover::DiscardWrites)
            .unwrap_err()
            .to_string();
        let expected = match defect {
            "expired_key" => "expired key",
            "extra_key" => "cardinality mismatch",
            _ => "value/TTL mismatch",
        };
        assert!(error.contains(expected), "{defect}: {error}");
    }
}

#[test]
fn rehearsal_requires_the_destination_write_to_change_state() {
    use hydracache_cluster_testkit::value_plane_service_075::ServiceTtl;

    let mut dry = ready_fixture();
    dry.shadow_reads(7).unwrap();
    let entry = dry.staging().diagnostic_snapshot(7).unwrap().remove(0);
    let mut delete = ServiceCommand::point("delete", &entry.key);
    delete.now = 7;
    delete.identity = Some(MutationIdentity::new("migration-rehearsal", 1));
    dry.staging().execute(delete).unwrap();
    let mut restore = ServiceCommand::point("set", &entry.key);
    restore.now = 7;
    restore.identity = Some(MutationIdentity::new("restore-fixture", 1));
    restore.value = Some(entry.value);
    restore.ttl = entry
        .remaining_ttl
        .map_or(ServiceTtl::Eternal, ServiceTtl::ExpireAfter);
    dry.staging().execute(restore).unwrap();
    assert!(dry
        .rehearse(7, PostCutover::DiscardWrites)
        .unwrap_err()
        .to_string()
        .contains("did not change the destination"));
}

#[test]
fn rehearsal_cli_requires_explicit_policy_and_retains_each_fixture_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let fixtures = temp.path().join("fixtures.json");
    std::fs::write(&fixtures, CORPUS).unwrap();
    let output = temp.path().join("rehearsal.json");
    let run = |extra: &[&str]| {
        let mut args = vec![
            "--release".into(),
            "0.75".into(),
            "--fixtures".into(),
            fixtures.display().to_string(),
            "--output".into(),
            output.display().to_string(),
        ];
        args.extend(extra.iter().map(|value| value.to_string()));
        xtask::imap_migration_dry_run::run(args)
    };
    for invalid in [
        vec!["--rehearse-at", "7"],
        vec!["--post-cutover", "read_only"],
        vec!["--rehearse-at", "x", "--post-cutover", "read_only"],
        vec!["--rehearse-at", "7", "--post-cutover", "drop_everything"],
    ] {
        assert!(run(&invalid).is_err());
        assert!(!output.exists());
    }
    run(&["--rehearse-at", "7", "--post-cutover", "discard_writes"]).unwrap();
    let receipt: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(receipt["production_import"], false);
    for fixture in receipt["fixtures"].as_array().unwrap() {
        let rehearsal = &fixture["rehearsal"];
        assert_eq!(rehearsal["fixed_tick"], 7);
        assert_eq!(rehearsal["post_cutover"], "discard_writes");
        assert_eq!(rehearsal["destination_write_applied"], true);
        assert_eq!(
            rehearsal["rollback_digest"],
            rehearsal["pre_cutover_digest"]
        );
    }
    for policy in ["read_only", "refuse_writes"] {
        let output = temp.path().join(format!("{policy}.json"));
        xtask::imap_migration_dry_run::run(vec![
            "--release".into(),
            "0.75".into(),
            "--rehearse-at".into(),
            "7".into(),
            "--post-cutover".into(),
            policy.into(),
            "--output".into(),
            output.display().to_string(),
        ])
        .unwrap();
        let receipt: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
        assert_eq!(receipt["fixtures"][0]["rehearsal"]["post_cutover"], policy);
    }
}
