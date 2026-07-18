use hydracache_cache_sim::{
    parse_trace, trace_digest, KeyDistribution, KeyScheduleSpec, TraceCatalogId,
    KEY_SCHEDULE_GENERATOR_VERSION,
};

#[test]
fn seeded_uniform_key_schedule_is_versioned_reproducible_and_digest_bound() {
    let spec = KeyScheduleSpec::uniform(67, 17, 24);
    let first = spec.generate().expect("valid uniform schedule");
    let second = spec.generate().expect("same schedule");

    assert_eq!(spec.generator_version, KEY_SCHEDULE_GENERATOR_VERSION);
    assert_eq!(first, second);
    assert_eq!(first.keys.len(), 24);
    assert!(first.keys.iter().all(|key| *key < 17));
    assert_eq!(
        first.keys,
        [16, 16, 9, 11, 3, 8, 1, 6, 6, 7, 6, 3, 5, 7, 10, 1, 3, 11, 1, 0, 13, 3, 9, 10,]
    );
    assert_eq!(
        first.digest,
        "c6ea0714ea8750c86b7b97e1bdfc8f6436fc51d7fc0f1bdf5de810f4f0103e8c"
    );

    let different_seed = KeyScheduleSpec::uniform(68, 17, 24)
        .generate()
        .expect("different-seed schedule");
    assert_ne!(first.keys, different_seed.keys);
    assert_ne!(first.digest, different_seed.digest);
}

#[test]
fn seeded_zipfian_schedule_is_reproducible_skewed_and_records_theta() {
    let spec = KeyScheduleSpec::zipfian(6701, 100, 50_000, 0.99);
    let first = spec.generate().expect("valid Zipfian schedule");
    let second = spec.generate().expect("same Zipfian schedule");
    assert_eq!(first, second);
    assert!(first.keys.iter().all(|key| *key < 100));

    let hottest_decile = first.keys.iter().filter(|key| **key < 10).count();
    let coldest_decile = first.keys.iter().filter(|key| **key >= 90).count();
    assert!(
        hottest_decile > coldest_decile.saturating_mul(5),
        "expected visible rank skew: hottest={hottest_decile}, coldest={coldest_decile}"
    );

    let changed_theta = KeyScheduleSpec::zipfian(6701, 100, 50_000, 0.75)
        .generate()
        .expect("changed-theta schedule");
    assert_ne!(first.digest, changed_theta.digest);
    assert!(matches!(
        first.spec.distribution,
        KeyDistribution::Zipfian { theta } if theta == 0.99
    ));

    let short = KeyScheduleSpec::zipfian(6701, 10, 16, 0.99)
        .generate()
        .expect("short golden schedule");
    assert_eq!(short.keys, [4, 0, 6, 1, 3, 8, 3, 5, 0, 1, 1, 1, 8, 7, 3, 0]);
    assert_eq!(
        short.digest,
        "6a013f340569bd968066d4800bb7a973bfcafa62cc50a383e60ffd7e01032364"
    );
}

#[test]
fn key_schedule_rejects_unversioned_or_vacuous_inputs() {
    let mut wrong_version = KeyScheduleSpec::uniform(1, 10, 10);
    wrong_version.generator_version = KEY_SCHEDULE_GENERATOR_VERSION + 1;
    assert!(wrong_version.generate().is_err());
    assert!(KeyScheduleSpec::uniform(1, 0, 10).generate().is_err());
    assert!(KeyScheduleSpec::uniform(1, 10, 0).generate().is_err());
    assert!(KeyScheduleSpec::zipfian(1, 10, 10, 0.0).generate().is_err());
    assert!(KeyScheduleSpec::zipfian(1, 10, 10, f64::NAN)
        .generate()
        .is_err());
}

#[test]
fn w22_trace_catalog_reuses_exact_sources_and_records_order_sensitive_digests() {
    let expected = [
        (
            TraceCatalogId::Standard,
            25,
            "7e61aa4db4f6600497a587ac53723585d049987e5f3544c7a596bd740d926605",
            "23cdede2d99cfa41a79f37dabeeea169e1a8eb0d664beebe3f1d8bd106d14d15",
        ),
        (
            TraceCatalogId::SkewedZipfian,
            34,
            "d9245cfb0c0c1267ffab9098d0e0e6103a9aea4b4fbf618a7a1e5b8aae4564e2",
            "3cdb855ad2a24fe23d45198925fb88566642a00e0c97ccc9f3089a350696706a",
        ),
        (
            TraceCatalogId::RecencyTtl,
            24,
            "f434691f281e9bbe0ef2b1c2f77d19b158964ea6c39b0370d0b7743d74502e3a",
            "5ce1c862b0bfb7fe76699a6ff474a0f2358ba60fdf0e52dc385a15f269e1296b",
        ),
    ];
    assert_eq!(
        TraceCatalogId::ALL,
        expected.map(|(id, _, _, _)| id),
        "catalog order is part of the shared input contract"
    );

    for (id, event_count, source_digest, event_digest) in expected {
        let committed = id.load().expect("committed trace parses");
        assert_eq!(committed.id, id);
        assert_eq!(committed.events.len(), event_count);
        assert_eq!(committed.source_digest, source_digest);
        assert_eq!(committed.event_digest, event_digest);
        assert_eq!(
            committed.events,
            parse_trace(id.source()).expect("public source parses identically")
        );
        assert_eq!(committed.event_digest, trace_digest(&committed.events));

        let mut reversed = committed.events.clone();
        reversed.reverse();
        assert_ne!(
            committed.event_digest,
            trace_digest(&reversed),
            "{id} digest must bind replay order"
        );
    }
}

#[test]
fn trace_digest_binds_timestamps_keys_and_field_boundaries() {
    let original = TraceCatalogId::Standard.load().unwrap().events;
    let original_digest = trace_digest(&original);

    let mut changed_timestamp = original.clone();
    changed_timestamp[0].at = changed_timestamp[0].at.saturating_add(1);
    assert_ne!(original_digest, trace_digest(&changed_timestamp));

    let mut changed_key = original.clone();
    changed_key[0].key.push('x');
    assert_ne!(original_digest, trace_digest(&changed_key));
}
