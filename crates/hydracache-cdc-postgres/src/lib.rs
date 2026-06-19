use hydracache_db::InvalidationIntent;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationOffset {
    value: String,
}

impl ReplicationOffset {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdcOperation {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticChangeEvent {
    pub table: String,
    pub operation: CdcOperation,
    pub key: String,
    pub offset: ReplicationOffset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdcIntentMapping {
    table: String,
    entity: String,
}

impl CdcIntentMapping {
    pub fn entity(table: impl Into<String>, entity: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            entity: entity.into(),
        }
    }

    pub fn map(&self, event: &SyntheticChangeEvent) -> Option<InvalidationIntent> {
        if event.table == self.table {
            Some(InvalidationIntent::entity(&self.entity, &event.key))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PostgresCdcConnector {
    slot: String,
    last_offset: Option<ReplicationOffset>,
}

impl PostgresCdcConnector {
    pub fn deferred(slot: impl Into<String>) -> Self {
        Self {
            slot: slot.into(),
            last_offset: None,
        }
    }

    pub fn slot(&self) -> &str {
        &self.slot
    }

    pub fn last_offset(&self) -> Option<&ReplicationOffset> {
        self.last_offset.as_ref()
    }

    pub async fn next_intents(
        &mut self,
    ) -> Result<(Vec<InvalidationIntent>, ReplicationOffset), CdcError> {
        Err(CdcError::NotImplemented)
    }
}

#[derive(Debug, Error)]
pub enum CdcError {
    #[error(
        "Postgres logical replication connector is deferred; HydraCache 0.38 ships only value-free intent mapping"
    )]
    NotImplemented,
}
