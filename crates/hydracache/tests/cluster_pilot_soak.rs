use hydracache::testing::StagingClusterHarness;

#[tokio::test]
#[ignore = "pilot soak is an on-demand release gate; run with --ignored --nocapture"]
async fn cluster_pilot_soak() {
    let outcome = StagingClusterHarness::builder()
        .cluster_name("pilot-soak")
        .members(3)
        .clients(6)
        .invalidations(10_000)
        .build()
        .await
        .run_full_gate()
        .await;

    println!("{}", serde_json::to_string_pretty(&outcome.report).unwrap());

    assert_eq!(outcome.report.decode_errors, 0);
    assert_eq!(outcome.report.publish_failures, 0);
    assert_eq!(outcome.report.receiver_closed, 0);
    assert!(outcome.report.lagged <= 5);
    assert_eq!(outcome.report.published, outcome.report.applied);
    assert!(outcome.report.stale_generation_rejected > 0);
    assert!(outcome.report.remote_fetch_success > 0);
}
