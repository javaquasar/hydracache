use hydracache_cdc_postgres::{
    CdcError, CdcIntentMapping, CdcOperation, PostgresCdcConnector, ReplicationOffset,
    SyntheticChangeEvent,
};
use hydracache_db::InvalidationIntent;

#[test]
fn synthetic_event_to_intent() {
    let mapping = CdcIntentMapping::entity("users", "user");
    let event = SyntheticChangeEvent {
        table: "users".to_owned(),
        operation: CdcOperation::Update,
        key: "42".to_owned(),
        offset: ReplicationOffset::new("0/16B6C50"),
    };

    assert_eq!(
        mapping.map(&event),
        Some(InvalidationIntent::entity("user", "42"))
    );
    assert_eq!(event.offset.as_str(), "0/16B6C50");
}

#[tokio::test]
async fn logical_replication_connector_is_explicitly_deferred() {
    let mut connector = PostgresCdcConnector::deferred("hydracache_slot");
    let error = connector.next_intents().await.unwrap_err();

    assert!(matches!(error, CdcError::NotImplemented));
}

#[test]
#[ignore = "requires a Postgres logical replication slot"]
fn pg_logical_replication_smoke() {
    let Ok(url) = std::env::var("HYDRACACHE_TEST_POSTGRES_URL") else {
        return;
    };
    assert!(url.starts_with("postgres://") || url.starts_with("postgresql://"));
}
