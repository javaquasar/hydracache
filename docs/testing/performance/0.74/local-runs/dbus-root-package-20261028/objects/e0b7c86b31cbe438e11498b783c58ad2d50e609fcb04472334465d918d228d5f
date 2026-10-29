//! Read-only parent view for exclusive sinks; no ancestry replacement or opener.
use super::*;
impl CollectorReservationStorage<'_> {
    pub(crate) fn collector_output_parent(&self) -> Result<&std::fs::File, StorageAttemptError> {
        if self.is_refused() || !self.starting.as_ref().is_some_and(|s| s.acknowledged) {
            return Err(StorageAttemptError::Refused);
        }
        super::super::super::parent(self.original.empty.storage).map_err(Into::into)
    }
}
