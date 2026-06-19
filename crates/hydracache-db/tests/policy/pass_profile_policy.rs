use hydracache_db::{query_cache_policy, DimensionValidationMode, ProfileValidation};

fn main() {
    let policy = query_cache_policy!(
        name = "search-users",
        key_segments = ["tenant", 7_u64, "permission", "abc", "q", "ada", "page", 1_u64, "sort", "name"],
        tag_segments = [["tenant", 7_u64], ["permission", "abc"], ["q", "ada"], ["page", 1_u64], ["sort", "name"]],
        profile = tenant_permission_search,
        dimension_validation = deny,
        ttl_secs = 30,
    );

    assert_eq!(policy.validate_dimension_profile(), ProfileValidation::Pass);
    assert_eq!(
        policy.dimension_validation_mode(),
        DimensionValidationMode::Deny
    );
}
