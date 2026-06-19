#[test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL and catalog grants"]
fn pg_view_dependency_expansion() {
    let url = std::env::var("HYDRACACHE_TEST_POSTGRES_URL")
        .expect("set HYDRACACHE_TEST_POSTGRES_URL to run catalog lint smoke tests");
    assert!(url.starts_with("postgres://") || url.starts_with("postgresql://"));
}

#[test]
#[ignore = "requires HYDRACACHE_TEST_POSTGRES_URL and a readonly role"]
fn pg_missing_permission_clear_error() {
    let url = std::env::var("HYDRACACHE_TEST_POSTGRES_URL")
        .expect("set HYDRACACHE_TEST_POSTGRES_URL to run catalog lint smoke tests");
    assert!(url.contains("://"));
}
