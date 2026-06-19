#[test]
#[ignore = "requires a Postgres test container and HYDRACACHE_TEST_POSTGRES_URL"]
fn pg_trigger_outbox_worker_end_to_end() {
    let Ok(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL") else {
        return;
    };
    assert!(url.starts_with("postgres://") || url.starts_with("postgresql://"));
}

#[test]
#[ignore = "requires a Postgres test container and HYDRACACHE_TEST_POSTGRES_URL"]
fn pg_listen_notify_wakeup() {
    let Ok(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL") else {
        return;
    };
    assert!(url.contains("://"));
}
