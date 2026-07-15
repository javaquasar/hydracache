use hydracache_cache_sim::{belady_optimal, parse_trace, replay_policy, PolicyKind};

#[test]
fn eviction_hit_rate_is_within_tolerance_of_belady_optimum_on_standard_traces() {
    let trace = parse_trace(include_str!("../traces/standard.trace")).expect("valid trace");
    let belady = belady_optimal(&trace, 3);
    let hydra = replay_policy(&trace, 3, None, PolicyKind::Hydra);

    assert!(
        hydra.hit_rate() >= belady.hit_rate() * 0.80,
        "hydra hit-rate {:.3} must stay within tolerance of Belady {:.3}",
        hydra.hit_rate(),
        belady.hit_rate()
    );
}

#[test]
fn eviction_beats_lru_and_lfu_baselines_on_skewed_zipfian_trace() {
    let trace = parse_trace(include_str!("../traces/skewed_zipfian.trace")).expect("valid trace");
    let hydra = replay_policy(&trace, 3, None, PolicyKind::Hydra);
    let lru = replay_policy(&trace, 3, None, PolicyKind::Lru);
    let lfu = replay_policy(&trace, 3, None, PolicyKind::Lfu);

    assert!(
        hydra.hit_rate() > lru.hit_rate(),
        "hydra {:.3} must beat LRU {:.3}",
        hydra.hit_rate(),
        lru.hit_rate()
    );
    assert!(
        hydra.hit_rate() > lfu.hit_rate(),
        "hydra {:.3} must beat LFU {:.3}",
        hydra.hit_rate(),
        lfu.hit_rate()
    );
}

#[test]
fn ttl_expiry_does_not_collapse_hit_rate_under_recency_skew() {
    let trace = parse_trace(include_str!("../traces/recency_ttl.trace")).expect("valid trace");
    let no_ttl = replay_policy(&trace, 4, None, PolicyKind::Hydra);
    let ttl = replay_policy(&trace, 4, Some(12), PolicyKind::Hydra);

    assert!(
        ttl.hit_rate() >= no_ttl.hit_rate() - 0.20,
        "ttl hit-rate {:.3} should not collapse from no-ttl {:.3}",
        ttl.hit_rate(),
        no_ttl.hit_rate()
    );
    assert!(
        ttl.hit_rate() >= 0.60,
        "recency-skew ttl trace should retain useful hit-rate, got {:.3}",
        ttl.hit_rate()
    );
}

#[test]
fn canary_random_eviction_policy_fails_the_hit_rate_bound() {
    let trace = parse_trace(include_str!("../traces/standard.trace")).expect("valid trace");
    let belady = belady_optimal(&trace, 3);
    let random = replay_policy(&trace, 3, None, PolicyKind::Random);

    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W22") {
        assert!(
            random.hit_rate() >= belady.hit_rate() * 0.80,
            "HC-CANARY-RED:W22 eviction policy fell below Belady tolerance"
        );
    }

    assert!(
        random.hit_rate() < belady.hit_rate() * 0.80,
        "canary random policy must violate the Belady tolerance: random {:.3}, Belady {:.3}",
        random.hit_rate(),
        belady.hit_rate()
    );
}
