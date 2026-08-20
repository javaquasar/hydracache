use xtask::memory_statistics::{
    hodges_lehmann_paired, moving_block_slope_interval, practical_verdict,
    require_final_checkpoint, theil_sen_slope, validated_cumulative_counter, ConfidenceInterval,
    TimeSample, ESTIMATOR_VERSION,
};

fn series(values: &[f64]) -> Vec<TimeSample> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| TimeSample {
            sequence: index as u64,
            monotonic_seconds: index as f64 * 5.0,
            value: *value,
        })
        .collect()
}

#[test]
fn golden_flat_growth_and_plateau_series() {
    assert_eq!(ESTIMATOR_VERSION, "memory-statistics-071-v1");
    assert_eq!(
        theil_sen_slope(&series(&[10.0, 10.0, 10.0, 10.0])).unwrap(),
        0.0
    );
    assert_eq!(
        theil_sen_slope(&series(&[0.0, 5.0, 10.0, 15.0])).unwrap(),
        1.0
    );
    let plateau = theil_sen_slope(&series(&[0.0, 10.0, 20.0, 20.0, 20.0, 20.0])).unwrap();
    assert!(plateau > 0.0 && plateau < 1.0);
}

#[test]
fn deterministic_moving_block_bootstrap_preserves_autocorrelation_contract() {
    let autocorrelated = series(&[
        100.0, 101.0, 103.0, 106.0, 110.0, 115.0, 121.0, 128.0, 136.0, 145.0, 155.0, 166.0, 178.0,
        191.0,
    ]);
    let first = moving_block_slope_interval(&autocorrelated, 4, 1000, 710071, 0.95).unwrap();
    let second = moving_block_slope_interval(&autocorrelated, 4, 1000, 710071, 0.95).unwrap();
    assert_eq!(first, second);
    assert!(first.lower <= first.upper);
}

#[test]
fn row_order_does_not_change_estimate_but_clock_regression_fails() {
    let ordered = series(&[0.0, 2.0, 4.0, 6.0, 8.0]);
    let mut shuffled = vec![ordered[3], ordered[0], ordered[4], ordered[1], ordered[2]];
    assert_eq!(theil_sen_slope(&ordered), theil_sen_slope(&shuffled));
    shuffled[2].monotonic_seconds = 1.0;
    assert!(theil_sen_slope(&shuffled)
        .expect_err("clock regression")
        .contains("clock regressed"));
}

#[test]
fn missing_final_checkpoint_and_counter_reset_fail_loud() {
    let samples = series(&[1.0, 2.0, 3.0, 4.0]);
    assert!(require_final_checkpoint(&samples, 4)
        .expect_err("missing final")
        .contains("missing final checkpoint"));
    assert!(validated_cumulative_counter(&series(&[1.0, 3.0, 2.0, 4.0]))
        .expect_err("counter reset")
        .contains("counter reset"));
}

#[test]
fn extreme_valid_sample_is_retained_and_paired_estimator_is_robust() {
    let ordinary = theil_sen_slope(&series(&[0.0, 1.0, 2.0, 3.0, 4.0])).unwrap();
    let with_extreme = theil_sen_slope(&series(&[0.0, 1.0, 200.0, 3.0, 4.0])).unwrap();
    assert_eq!(ordinary, with_extreme);
    assert_eq!(
        hodges_lehmann_paired(&[100.0, 110.0, 120.0], &[90.0, 100.0, 110.0]).unwrap(),
        -10.0
    );
}

#[test]
fn confidence_interval_crossing_practical_limit_is_not_green() {
    let crossing = ConfidenceInterval {
        estimate: -8.0,
        lower: -20.0,
        upper: 1.0,
        samples: 10,
        seed: 710071,
        iterations: 1000,
        block_samples: 4,
    };
    assert!(!practical_verdict(crossing, 100.0, 5.0, 0.05, true).unwrap());
    let clear = ConfidenceInterval {
        upper: -6.0,
        ..crossing
    };
    assert!(practical_verdict(clear, 100.0, 5.0, 0.05, true).unwrap());
}
