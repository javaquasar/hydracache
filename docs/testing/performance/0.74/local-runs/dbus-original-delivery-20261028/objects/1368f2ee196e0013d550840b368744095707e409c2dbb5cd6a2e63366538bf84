//! Original owned departure document; no replacement, adoption or execution.
use super::{open_at, ConsumptionDocument, PublicationStorage, StoragePublicationError};

pub(super) const NAME: &str = "diagnostic-starting-intent-074.json";
pub(crate) const MAX_STARTING_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartingStage {
    BeforeCreate,
    Created,
    PrefixWritten,
    BytesWritten,
    Sealed,
    FileSynced,
    DirectorySynced,
    Acknowledging,
}
impl PublicationStorage<'_> {
    pub(crate) fn has_owned_attempt(&self) -> bool {
        self.attempt.is_some()
    }
    pub(crate) fn initial_record_sha256(&mut self) -> Result<String, StoragePublicationError> {
        self.revalidate_initial_consumption()?;
        Ok(self.consumption_documents[0]
            .as_ref()
            .ok_or(StoragePublicationError::Refused)?
            .raw_sha256())
    }
    fn starting_boundary<E: From<StoragePublicationError>>(
        &mut self,
        at: StartingStage,
        checkpoint: &mut impl FnMut(StartingStage) -> Result<(), E>,
    ) -> Result<(), E> {
        self.revalidate_initial_consumption()?;
        checkpoint(at)?;
        self.revalidate_initial_consumption()?;
        Ok(())
    }
    pub(crate) fn publish_starting_with<E: From<StoragePublicationError>>(
        &mut self,
        raw: &[u8],
        mut checkpoint: impl FnMut(StartingStage) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.is_refused()
            || !self.has_owned_attempt()
            || !self.consumption_acknowledged
            || self.starting_attempted
            || raw.is_empty()
            || raw.len() > MAX_STARTING_BYTES
        {
            return Err(StoragePublicationError::Refused.into());
        }
        self.starting_attempted = true;
        self.starting_boundary(StartingStage::BeforeCreate, &mut checkpoint)?;
        // The exact newly created descriptor is retained before namespace checks.
        self.starting_document = Some(ConsumptionDocument::create(
            self.leaf()?,
            NAME,
            MAX_STARTING_BYTES,
        )?);
        self.starting_boundary(StartingStage::Created, &mut checkpoint)?;
        let parent = open_at(self.leaf()?, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)
            .map_err(StoragePublicationError::from)?;
        let middle = raw.len() / 2;
        for (bytes, at, seal) in [
            (&raw[..middle], StartingStage::PrefixWritten, false),
            (&raw[middle..], StartingStage::BytesWritten, false),
            (&[][..], StartingStage::Sealed, true),
        ] {
            self.revalidate_initial_consumption()?;
            self.starting_document
                .as_mut()
                .ok_or(StoragePublicationError::Refused)?
                .append_owned(&parent, bytes, seal)?;
            self.starting_boundary(at, &mut checkpoint)?;
        }
        self.starting_document
            .as_ref()
            .ok_or(StoragePublicationError::Refused)?
            .sync_owned(&parent)?;
        self.starting_boundary(StartingStage::FileSynced, &mut checkpoint)?;
        self.leaf()?
            .sync_all()
            .map_err(StoragePublicationError::from)?;
        self.starting_boundary(StartingStage::DirectorySynced, &mut checkpoint)?;
        self.starting_boundary(StartingStage::Acknowledging, &mut checkpoint)?;
        self.starting_acknowledged = true;
        Ok(())
    }
    pub(crate) fn revalidate_starting(&mut self) -> Result<(), StoragePublicationError> {
        if !self.starting_acknowledged {
            return Err(StoragePublicationError::Refused);
        }
        self.revalidate_initial_consumption()
    }
}
