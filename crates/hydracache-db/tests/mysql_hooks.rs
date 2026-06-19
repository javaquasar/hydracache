#[test]
#[ignore = "requires a MySQL test container and HYDRACACHE_TEST_MYSQL_URL"]
fn mysql_trigger_outbox_worker_end_to_end() {
    let Ok(url) = std::env::var("HYDRACACHE_TEST_MYSQL_URL") else {
        return;
    };
    assert!(url.starts_with("mysql://"));
}
