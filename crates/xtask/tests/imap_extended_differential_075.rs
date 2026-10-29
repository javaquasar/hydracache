use hydracache_cluster_testkit::value_plane_extended_075::{ExtendedCorpus, MAX_LINE, MAX_OUTPUT};
use serde_json::{json, Value};
use xtask::imap_extended_differential_075::{
    execute_java, execute_rust_process, generate, parse_observations, retain_corpus,
};

fn corpus() -> ExtendedCorpus {
    ExtendedCorpus::parse("imap-extended-semantic-v2\t117\t1\n61\tresources\t~\t~\t~\t0\t0\n")
        .unwrap()
}
fn observation() -> Value {
    json!({"result":{"kind":"resources","value":null,"ttl":{"kind":"absent","remaining_ticks":0},"items":[],"statuses":[],"number":0},"snapshot":[],"events":[],"token_relations":{},"owners":{"outcomes":0,"pending":0,"listeners":1,"snapshot_payload":false},"tick":0})
}
fn transcript(value: &Value, expected: bool) -> Vec<u8> {
    format!(
        "imap-extended-observations-v2\t{}\n61\t{value}\n",
        if expected { "expected" } else { "actual" }
    )
    .into_bytes()
}

#[test]
fn closed_v2_observation_schema_preserves_optional_bytes_ttl_statuses_and_unicode() {
    let c = corpus();
    assert_eq!(
        parse_observations(&transcript(&observation(), false), &c, false).unwrap(),
        vec![observation()]
    );
    let mut value = observation();
    value["result"]["value"] = json!("");
    value["result"]["ttl"] = json!({"kind":"eternal","remaining_ticks":0});
    value["result"]["items"] = json!([{"index":0,"key":"00ff","kind":"present","value":"","ttl":{"kind":"expiring","remaining_ticks":2}}]);
    value["result"]["statuses"] = json!([{"identity":{"client":"клиент\n\"","sequence":1},"kind":"completed","result":observation()["result"]}]);
    value["snapshot"] =
        json!([{"key":"","value":"00ff","ttl":{"kind":"eternal","remaining_ticks":0}}]);
    value["events"] =
        json!([{"kind":"added","key":"","old_value":null,"value":"00ff","watermark":1}]);
    value["token_relations"] = json!({"а=β":true});
    assert_eq!(
        parse_observations(&transcript(&value, true), &c, true).unwrap(),
        vec![value]
    );
    let empty = ExtendedCorpus::parse("imap-extended-semantic-v2\t0\t0\n").unwrap();
    assert!(
        parse_observations(b"imap-extended-observations-v2\texpected\n", &empty, true)
            .unwrap()
            .is_empty()
    );
    for text in ["\u{00a0}", "\u{0085}", "\u{2007}", "\u{202f}"] {
        let mut accepted = observation();
        accepted["result"]["kind"] = json!(text);
        assert!(parse_observations(&transcript(&accepted, false), &c, false).is_ok());
    }
}

#[test]
fn hostile_v2_json_rejects_unknown_missing_recursive_duplicate_or_wrongly_typed_fields() {
    let c = corpus();
    let mut variants = Vec::new();
    let mut v = observation();
    v["unknown"] = json!(0);
    variants.push(v);
    let mut v = observation();
    v.as_object_mut().unwrap().remove("tick");
    variants.push(v);
    for invalid in [
        json!(-1),
        json!(1.5),
        json!("0"),
        json!(u64::MAX),
        Value::Null,
    ] {
        let mut v = observation();
        v["tick"] = invalid;
        variants.push(v);
    }
    for invalid in [
        json!("AA"),
        json!("0"),
        json!(false),
        json!("00".repeat(4097)),
    ] {
        let mut v = observation();
        v["result"]["value"] = invalid;
        variants.push(v);
    }
    for ttl in [
        json!({"kind":"future","remaining_ticks":0}),
        json!({"kind":"expiring","remaining_ticks":0}),
        json!({"kind":"eternal","remaining_ticks":1}),
        json!({"kind":"absent"}),
    ] {
        let mut v = observation();
        v["result"]["ttl"] = ttl;
        variants.push(v);
    }
    for text in ["", "\u{001c}", "\u{2003}", "x=y", &"x".repeat(97)] {
        let mut v = observation();
        v["result"]["kind"] = json!(text);
        variants.push(v);
    }
    let status = json!({"identity":{"client":"c","sequence":1},"kind":"completed","result":observation()["result"]});
    let mut recursive = observation();
    recursive["result"]["statuses"] = json!([status.clone()]);
    let mut v = observation();
    v["result"]["statuses"] = json!([{"identity":{"client":"c","sequence":2},"kind":"completed","result":recursive["result"]}]);
    variants.push(v);
    let mut v = observation();
    v["result"]["statuses"] =
        json!([{"identity":{"client":"c","sequence":0},"kind":"unknown","result":null}]);
    variants.push(v);
    let mut v = observation();
    v["result"]["statuses"] = json!(vec![status; 65]);
    variants.push(v);
    let mut v = observation();
    v["owners"]["snapshot_payload"] = json!(0);
    variants.push(v);
    let mut v = observation();
    v["owners"]["pending"] = json!(-1);
    variants.push(v);
    let mut v = observation();
    v["events"] = json!([{"kind":"gap","key":"","old_value":null,"value":null,"watermark":0}]);
    variants.push(v);
    let mut v = observation();
    v["result"]["items"] = json!([{"index":2147483648_u64,"key":"","kind":"absent","value":null,"ttl":{"kind":"absent","remaining_ticks":0}}]);
    variants.push(v);
    let mut v = observation();
    v["token_relations"] = json!({"a=b":1});
    variants.push(v);
    let mut v = observation();
    v["token_relations"] = json!({"missing_delimiter":true});
    variants.push(v);
    let mut v = observation();
    v["token_relations"] = json!({("x".repeat(194)):true});
    variants.push(v);
    let mut v = observation();
    v["snapshot"] =
        json!([{"key":"00".repeat(1025),"value":"","ttl":{"kind":"eternal","remaining_ticks":0}}]);
    variants.push(v);
    let mut v = observation();
    v["events"] = json!(vec![
        json!({"kind":"gap","key":"","old_value":null,"value":null,"watermark":1});
        66
    ]);
    variants.push(v);
    for invalid in variants {
        assert!(parse_observations(&transcript(&invalid, false), &c, false).is_err());
    }
    let valid = String::from_utf8(transcript(&observation(), false)).unwrap();
    for invalid in [
        valid.replace("\"tick\":0", "\"tick\":0,\"tick\":1"),
        valid.replace("\"pending\":0", "\"pending\":0,\"pending\":0"),
        format!(
            "imap-extended-observations-v2\tactual\n61\t{}0{}\n",
            "[".repeat(130),
            "]".repeat(130)
        ),
    ] {
        assert!(parse_observations(invalid.as_bytes(), &c, false).is_err());
    }
}

#[test]
fn transcript_headers_identity_counts_encoding_and_allocation_bounds_fail_closed() {
    let c = corpus();
    let valid = String::from_utf8(transcript(&observation(), false)).unwrap();
    for invalid in [
        valid.replace("observations-v2", "observations-v3"),
        valid.replace("\tactual", "\texpected"),
        valid.replace("\n61\t", "\n62\t"),
        valid.replace('\n', "\r\n"),
        format!("{valid}61\t{{}}\n"),
        "imap-extended-observations-v2\tactual\n".to_owned(),
    ] {
        assert!(parse_observations(invalid.as_bytes(), &c, false).is_err());
    }
    assert!(parse_observations(&[0xff], &c, false).is_err());
    assert!(parse_observations(&vec![b'x'; MAX_OUTPUT + 1], &c, false).is_err());
    let long = format!(
        "imap-extended-observations-v2\tactual\n61\t{}\n",
        "x".repeat(MAX_LINE + 1)
    );
    assert!(parse_observations(long.as_bytes(), &c, false).is_err());
    let mut relations = serde_json::Map::new();
    for n in 0..4097 {
        relations.insert(format!("a={n}"), json!(true));
    }
    let mut huge = observation();
    huge["token_relations"] = relations.into();
    assert!(parse_observations(&transcript(&huge, false), &c, false).is_err());
}

#[test]
fn retained_v2_replay_artifacts_are_content_addressed_and_never_overwrite_conflicts() {
    let root = tempfile::tempdir().unwrap();
    let c = corpus();
    let path = retain_corpus(root.path(), &c).unwrap();
    assert!(path.starts_with(root.path().join("target/imap-075/differential-failures")));
    assert!(path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("extended-seed-117-"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), c.encode().unwrap());
    assert_eq!(retain_corpus(root.path(), &c).unwrap(), path);
    std::fs::write(&path, "existing different replay").unwrap();
    assert!(retain_corpus(root.path(), &c).is_err());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "existing different replay"
    );
    let mut invalid = c;
    invalid.steps.push(invalid.steps[0].clone());
    assert!(retain_corpus(root.path(), &invalid).is_err());
}

#[test]
fn v2_process_setup_and_generation_bounds_refuse_missing_actual_peer() {
    let root = tempfile::tempdir().unwrap();
    assert!(generate(root.path(), 117, 0).is_err());
    assert!(generate(root.path(), 117, 257).is_err());
    assert!(generate(root.path(), 117, 1).is_err());
    assert!(execute_java(root.path(), &corpus(), false, None).is_err());
    assert!(execute_java(root.path(), &corpus(), true, Some("drop_event")).is_err());
    let mut invalid = corpus();
    invalid.steps[0].id = "".into();
    assert!(parse_observations(&transcript(&observation(), false), &invalid, false).is_err());
    assert!(execute_rust_process(root.path(), &invalid, false).is_err());
    let configured = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.path().join("target"));
    let target = if configured.is_absolute() {
        configured
    } else {
        root.path().join(configured)
    };
    let binary = target.join("debug").join(if cfg!(windows) {
        "imap-extended-owner-075.exe"
    } else {
        "imap-extended-owner-075"
    });
    if !binary.is_file() {
        assert!(execute_rust_process(root.path(), &corpus(), false)
            .unwrap_err()
            .to_string()
            .contains("peer is missing"));
    }
}

#[test]
fn extended_cli_rejects_ambiguous_options_and_invalid_replays_before_build_or_owner_execution() {
    for args in [
        vec![],
        vec!["--release"],
        vec!["--release", "0.74"],
        vec!["--release", "0.75", "--unknown", "x"],
        vec!["--release", "0.75", "--release", "0.75"],
        vec!["--release", "0.75", "--seed", "+1"],
        vec!["--release", "0.75", "--seed", "-0"],
        vec!["--release", "0.75", "--seed", "9223372036854775808"],
        vec!["--release", "0.75", "--seed", "1", "--script", "unused.tsv"],
    ] {
        assert!(xtask::imap_extended_differential_075::run(
            args.into_iter().map(str::to_owned).collect()
        )
        .is_err());
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid.tsv");
    std::fs::write(&path, "unsupported corpus").unwrap();
    let error = xtask::imap_extended_differential_075::run(vec![
        "--release".into(),
        "0.75".into(),
        "--script".into(),
        path.to_string_lossy().into(),
    ])
    .unwrap_err();
    assert!(error.to_string().contains("header"));
    assert!(!directory.path().join("target").exists());
}
