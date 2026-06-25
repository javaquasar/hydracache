use hydracache::{
    NamespacePersistenceRule, NamespacePersistenceSettings, PersistencePolicy,
    PersistenceRegionPlacement, RegionId, RegionSelector,
};

#[test]
fn region_persistence_namespace_persists_only_in_selected_regions() {
    let policy = PersistencePolicy::try_new([NamespacePersistenceRule::new(
        "wallet.*",
        NamespacePersistenceSettings::persistent()
            .with_region_selector(RegionSelector::only([RegionId::from("eu-west")])),
    )
    .unwrap()])
    .unwrap();
    let placement =
        PersistenceRegionPlacement::active_active("us-east", [RegionId::from("eu-west")]);

    let eu = policy
        .resolve_for_region("wallet.balance", &RegionId::from("eu-west"), &placement)
        .unwrap();
    let us = policy
        .resolve_for_region("wallet.balance", &RegionId::from("us-east"), &placement)
        .unwrap();

    assert!(eu.persists());
    assert!(!us.persists());
}

#[test]
fn region_persistence_persist_region_outside_placement_fails_loud() {
    let policy = PersistencePolicy::try_new([NamespacePersistenceRule::new(
        "wallet.*",
        NamespacePersistenceSettings::persistent()
            .with_region_selector(RegionSelector::only([RegionId::from("ap-south")])),
    )
    .unwrap()])
    .unwrap();
    let placement =
        PersistenceRegionPlacement::active_active("us-east", [RegionId::from("eu-west")]);

    let error = policy
        .resolve_for_region("wallet.balance", &RegionId::from("us-east"), &placement)
        .unwrap_err();

    assert!(error.to_string().contains("outside placement"));
    assert!(error.to_string().contains("ap-south"));
}

#[test]
fn region_persistence_home_region_only_selector_matches_placement() {
    let policy = PersistencePolicy::try_new([NamespacePersistenceRule::new(
        "cache.jwt.pem",
        NamespacePersistenceSettings::persistent()
            .with_region_selector(RegionSelector::HomeRegionOnly),
    )
    .unwrap()])
    .unwrap();
    let placement =
        PersistenceRegionPlacement::active_active("us-east", [RegionId::from("eu-west")]);

    let home = policy
        .resolve_for_region("cache.jwt.pem", &RegionId::from("us-east"), &placement)
        .unwrap();
    let peer = policy
        .resolve_for_region("cache.jwt.pem", &RegionId::from("eu-west"), &placement)
        .unwrap();

    assert!(home.persists());
    assert!(!peer.persists());
}
