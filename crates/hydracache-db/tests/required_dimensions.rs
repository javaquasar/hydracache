use hydracache_db::query_cache_policy;

#[test]
fn diagnostics_expose_required_dimension_labels() {
    let tenant_id = 7_u64;
    let permission_hash = "read";
    let query = "ada";
    let page = 1_u32;
    let sort = "name";

    let policy = query_cache_policy!(
        name = "search-users",
        key_segments = [
            "tenant",
            tenant_id,
            "permission",
            permission_hash,
            "users",
            "search",
            "query",
            query,
            "page",
            page,
            "sort",
            sort,
        ],
        required_dimensions = ["tenant", "permission", "query", "page", "sort"],
        ttl_secs = 30,
    );

    assert_eq!(policy.name(), Some("search-users"));
    assert_eq!(
        policy.required_dimensions_value(),
        &[
            "tenant".to_owned(),
            "permission".to_owned(),
            "query".to_owned(),
            "page".to_owned(),
            "sort".to_owned()
        ]
    );
}
