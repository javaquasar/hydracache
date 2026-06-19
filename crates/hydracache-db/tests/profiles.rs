use hydracache_db::{
    CustomProfile, DimensionAllow, DimensionAllowError, DimensionProfile, DimensionRequirement,
    ProfileValidation, QueryCachePolicy,
};

#[test]
fn profile_passes_when_linked() {
    let policy = QueryCachePolicy::named("search-users")
        .key("tenant:7:permission:abc:q:ada:page:1:sort:name")
        .with_key_dimension_labels(["tenant", "permission", "q", "page", "sort"])
        .with_tag_dimension_labels(["tenant", "permission", "q", "page", "sort"])
        .with_dimension_profile(DimensionProfile::TenantPermissionSearch);

    assert_eq!(policy.validate_dimension_profile(), ProfileValidation::Pass);
}

#[test]
fn fails_when_tenant_missing() {
    let policy = QueryCachePolicy::named("search-users")
        .key("permission:abc:q:ada:page:1:sort:name")
        .with_key_dimension_labels(["permission", "q", "page", "sort"])
        .with_tag_dimension_labels(["permission", "q", "page", "sort"])
        .with_dimension_profile(DimensionProfile::TenantPermissionSearch);

    assert_eq!(
        policy.validate_dimension_profile(),
        ProfileValidation::MissingDimensions(vec!["tenant".to_owned()])
    );
}

#[test]
fn fails_when_permission_missing() {
    let policy = QueryCachePolicy::named("search-users")
        .key("tenant:7:q:ada:page:1:sort:name")
        .with_key_dimension_labels(["tenant", "q", "page", "sort"])
        .with_tag_dimension_labels(["tenant", "q", "page", "sort"])
        .with_dimension_profile(DimensionProfile::TenantPermissionSearch);

    assert_eq!(
        policy.validate_dimension_profile(),
        ProfileValidation::MissingDimensions(vec!["permission".to_owned()])
    );
}

#[test]
fn fails_when_page_or_cursor_missing() {
    let paged = QueryCachePolicy::named("search")
        .key("q:ada:sort:name")
        .with_key_dimension_labels(["q", "sort"])
        .with_tag_dimension_labels(["q", "sort"])
        .with_dimension_profile(DimensionProfile::PagedSearch);
    let cursor = QueryCachePolicy::named("list")
        .key("users")
        .with_key_dimension_labels(["tenant"])
        .with_tag_dimension_labels(["tenant"])
        .with_dimension_profile(DimensionProfile::CursorList);

    assert_eq!(
        paged.validate_dimension_profile(),
        ProfileValidation::MissingDimensions(vec!["page".to_owned()])
    );
    assert_eq!(
        cursor.validate_dimension_profile(),
        ProfileValidation::MissingDimensions(vec!["cursor".to_owned()])
    );
}

#[test]
fn unlinked_label_is_rejected() {
    let policy = QueryCachePolicy::named("tenant-users")
        .key("tenant:7:users")
        .with_key_dimension_labels(["tenant"])
        .with_tag_dimension_labels(["users"])
        .with_dimension_profile(DimensionProfile::TenantScoped);

    assert_eq!(
        policy.validate_dimension_profile(),
        ProfileValidation::UnlinkedDimensions(vec!["tenant".to_owned()])
    );
}

#[test]
fn custom_profile_reused() {
    let profile = CustomProfile::new(
        "tenant-locale",
        [
            DimensionRequirement::linked("tenant"),
            DimensionRequirement::key_only("locale"),
        ],
    );
    let policy = QueryCachePolicy::named("localized")
        .key("tenant:7:locale:en")
        .with_key_dimension_labels(["tenant", "locale"])
        .with_tag_dimension_labels(["tenant"])
        .with_dimension_profile(DimensionProfile::Custom(profile));

    assert_eq!(policy.validate_dimension_profile(), ProfileValidation::Pass);
}

#[test]
fn allowlist_requires_reason_text() {
    let error = DimensionAllow::new("tenant", "").unwrap_err();

    assert_eq!(error, DimensionAllowError::EmptyReason);
}
