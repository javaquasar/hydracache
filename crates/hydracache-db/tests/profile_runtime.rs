use hydracache_db::{
    query_cache_policy, DimensionProfile, DimensionValidationMode, ProfileValidation,
    QueryCachePolicy,
};

#[test]
fn diagnostics_include_profile_and_required_labels() {
    let policy = query_cache_policy!(
        name = "search-users",
        key_segments = [
            "tenant",
            7_u64,
            "permission",
            "abc",
            "q",
            "ada",
            "page",
            1_u64,
            "sort",
            "name"
        ],
        tag_segments = [
            ["tenant", 7_u64],
            ["permission", "abc"],
            ["q", "ada"],
            ["page", 1_u64],
            ["sort", "name"]
        ],
        profile = tenant_permission_search,
        dimension_validation = deny,
        ttl_secs = 30,
    );

    assert!(matches!(
        policy.dimension_profile(),
        Some(DimensionProfile::TenantPermissionSearch)
    ));
    assert_eq!(
        policy.required_dimensions_value(),
        &[
            "tenant".to_owned(),
            "permission".to_owned(),
            "q".to_owned(),
            "page".to_owned(),
            "sort".to_owned()
        ]
    );
    assert_eq!(policy.validate_dimension_profile(), ProfileValidation::Pass);
}

#[test]
fn warn_mode_warns_does_not_fail() {
    let policy = QueryCachePolicy::named("tenant-users")
        .key("tenant:7:users")
        .with_key_dimension_labels(["tenant"])
        .with_dimension_profile(DimensionProfile::TenantScoped)
        .with_dimension_validation_mode(DimensionValidationMode::Warn);

    assert!(matches!(
        policy.validate_dimension_profile(),
        ProfileValidation::UnlinkedDimensions(_)
    ));
    policy.enforce_dimension_profile().unwrap();
}

#[test]
fn deny_mode_fails_gate() {
    let policy = QueryCachePolicy::named("tenant-users")
        .key("tenant:7:users")
        .with_key_dimension_labels(["tenant"])
        .with_dimension_profile(DimensionProfile::TenantScoped)
        .with_dimension_validation_mode(DimensionValidationMode::Deny);

    let error = policy.enforce_dimension_profile().unwrap_err();

    assert!(error.to_string().contains("dimension profile violation"));
}

#[test]
fn allowlist_reason_turns_violation_into_allowed_status() {
    let policy = QueryCachePolicy::named("tenant-users")
        .key("tenant:7:users")
        .with_key_dimension_labels(["tenant"])
        .with_dimension_profile(DimensionProfile::TenantScoped)
        .allow_dimension_violation("tenant", "legacy key reviewed")
        .unwrap()
        .with_dimension_validation_mode(DimensionValidationMode::Deny);

    assert!(matches!(
        policy.validate_dimension_profile(),
        ProfileValidation::Allowed { .. }
    ));
    policy.enforce_dimension_profile().unwrap();
}
