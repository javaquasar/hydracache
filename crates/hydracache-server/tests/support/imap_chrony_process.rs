use super::*;
use hydracache_client_hc2::{BatchItemState, BatchOperation};
use imap_hc2_campaign::owner_checkpoint;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn distributed_ttl_fences_a_stale_chrony_reference_through_owner_promotion() {
    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start_fixture(
        "distributed-imap-chrony",
        &pki(),
        MemberOptions {
            chrony_fixture: true,
            ..Default::default()
        },
    )
    .await;
    let http = w10h_http();
    let admin = cluster.spec.addresses[7];
    wait_imap_readiness(&http, admin, "ready").await;
    let key = cluster.key_for(0, Some(1), "od2");
    let batch_key = cluster.key_for(1, Some(2), "od2-batch-");
    let partition = fixture_imap_scope().locate(key.to_vec()).unwrap().0.value();
    let ttl = Duration::from_secs(6);
    let started = tokio::time::Instant::now();
    let value = Bytes::from_static(b"temporary");
    assert!(
        cluster.clients[1]
            .put(
                key.clone(),
                value.clone(),
                Some(ttl),
                Some(RequestOptions {
                    timeout: Duration::from_secs(2),
                    idempotency_key: Bytes::from_static(b"od2-finite-put"),
                }),
            )
            .await
            .unwrap()
            .applied
    );
    wait_w10i_get(&cluster.clients[2], key.clone(), Some(&value)).await;
    let before = owner_checkpoint(&http, &cluster.spec, [0, 1], partition, 1, 1).await;

    let feed = cluster.spec.chrony_feed.as_ref().unwrap();
    let update_feed = |mode| {
        let update = feed.with_extension("new");
        std::fs::write(&update, mode).unwrap();
        std::fs::rename(update, feed).unwrap();
    };
    update_feed("stale");
    wait_imap_readiness(&http, admin, "clock_uncertainty_exceeded").await;
    assert_eq!(
        cluster.clients[2]
            .get(key.clone(), None)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Unavailable
    );
    assert_eq!(
        cluster.clients[1]
            .put(batch_key.clone(), value.clone(), Some(ttl), None)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Unavailable
    );
    cluster.daemons[0].0.kill().unwrap();
    cluster.daemons[0].0.wait().unwrap();
    let promoted = owner_checkpoint(
        &http,
        &cluster.spec,
        [1, 2],
        partition,
        1,
        before.epoch() + 1,
    )
    .await;
    before.assert_same_image(&promoted);
    // Keep the clock fenced past the point at which the original TTL is due.
    tokio::time::sleep_until(started + ttl).await;
    assert_eq!(
        cluster.clients[2]
            .get(key.clone(), None)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Unavailable
    );
    update_feed("live");
    wait_w10i_get(&cluster.clients[2], key, None).await;
    let expired =
        owner_checkpoint(&http, &cluster.spec, [1, 2], partition, 2, promoted.epoch()).await;
    let batch = cluster.clients[2]
        .batch_outcomes(
            vec![BatchOperation::Put {
                key: batch_key.clone(),
                value: value.clone(),
                ttl: Some(ttl),
            }],
            None,
        )
        .await
        .unwrap();
    assert_eq!(batch.len(), 1);
    assert!(batch[0].receipt.as_ref().is_some_and(|receipt| {
        receipt.state == BatchItemState::Confirmed && receipt.applied.is_some()
    }));
    wait_w10i_get(&cluster.clients[1], batch_key, Some(&value)).await;
    println!(
        "OD-2 HC/2: stale chrony fenced TTL before and after promotion on partition {partition}; epochs {} -> {}; certified expiry prefix=2",
        before.epoch(),
        expired.epoch()
    );
}
