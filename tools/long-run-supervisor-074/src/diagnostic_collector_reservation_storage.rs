//! Original two-document storage; no reopening, replay, legacy transfer or execution.
use super::{open_at, stamp, AttemptStorage, Document, StorageAttemptError, CONTROLLER_NAME};
use std::os::fd::AsRawFd;

#[path = "diagnostic_collector_starting_storage.rs"]
pub(crate) mod starting_storage;

pub(crate) const NAME: &str = "diagnostic-collector-reserved-074.json";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectorReservedStage {
    BeforeCreate,
    Created,
    CreateDirectorySynced,
    PrefixWritten,
    BytesWritten,
    Sealed,
    FileSynced,
    DirectorySynced,
    Acknowledging,
}
pub(crate) struct CollectorReservationStorage<'storage> {
    original: AttemptStorage<'storage>,
    document: Option<Document>,
    attempted: bool,
    acknowledged: bool,
    starting: Option<starting_storage::StartingPhase>,
}
impl<'storage> CollectorReservationStorage<'storage> {
    pub(crate) fn from_original(original: AttemptStorage<'storage>) -> Self {
        Self {
            original,
            document: None,
            attempted: false,
            acknowledged: false,
            starting: None,
        }
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.original.is_refused()
    }
    pub(crate) fn latch_refusal(&mut self) {
        self.original.latch_refusal();
    }

    fn inspect(&mut self) -> Result<(), StorageAttemptError> {
        if self.is_refused() || !self.original.acknowledged || self.original.name != CONTROLLER_NAME
        {
            return Err(StorageAttemptError::Refused);
        }
        self.original.ancestry()?;
        let leaf = self.original.leaf()?;
        let original = self
            .original
            .document
            .as_ref()
            .ok_or(StorageAttemptError::Refused)?;
        original.inspect(leaf)?;
        if let Some(doc) = &self.document {
            doc.inspect(leaf)?;
        }
        let fresh = open_at(leaf, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let mut seen = [false; 2];
        for entry in std::fs::read_dir(format!("/proc/self/fd/{}", fresh.as_raw_fd()))? {
            let name = entry?.file_name();
            let index = if name == CONTROLLER_NAME {
                0
            } else if self.document.is_some() && name == NAME {
                1
            } else {
                return Err(StorageAttemptError::Context);
            };
            if seen[index] {
                return Err(StorageAttemptError::Context);
            }
            seen[index] = true;
        }
        if seen != [true, self.document.is_some()] {
            return Err(StorageAttemptError::Context);
        }
        original.inspect(leaf)?;
        if let Some(doc) = &self.document {
            doc.inspect(leaf)?;
        }
        self.original.ancestry()
    }
    fn boundary<E: From<StorageAttemptError>>(
        &mut self,
        at: CollectorReservedStage,
        checkpoint: &mut impl FnMut(CollectorReservedStage) -> Result<(), E>,
    ) -> Result<(), E> {
        self.inspect()?;
        checkpoint(at)?;
        self.inspect()?;
        Ok(())
    }
    pub(crate) fn publish_with<E: From<StorageAttemptError>>(
        &mut self,
        raw: &[u8],
        mut checkpoint: impl FnMut(CollectorReservedStage) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.attempted || self.is_refused() {
            return Err(StorageAttemptError::Refused.into());
        }
        self.attempted = true;
        if raw.is_empty() || raw.len() > super::MAX_RECORD_BYTES {
            return Err(StorageAttemptError::Document.into());
        }
        self.boundary(CollectorReservedStage::BeforeCreate, &mut checkpoint)?;
        let file = open_at(
            self.original.leaf()?,
            NAME,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )
        .map_err(StorageAttemptError::from)?;
        self.document = Some(Document {
            name: NAME,
            stamp: stamp(&file.metadata().map_err(StorageAttemptError::from)?),
            file,
            raw: Vec::new(),
            mode: 0o600,
        });
        self.boundary(CollectorReservedStage::Created, &mut checkpoint)?;
        self.original
            .leaf()?
            .sync_all()
            .map_err(StorageAttemptError::from)?;
        self.boundary(
            CollectorReservedStage::CreateDirectorySynced,
            &mut checkpoint,
        )?;
        let middle = raw.len() / 2;
        for (bytes, seal, at) in [
            (&raw[..middle], false, CollectorReservedStage::PrefixWritten),
            (&raw[middle..], false, CollectorReservedStage::BytesWritten),
            (&[][..], true, CollectorReservedStage::Sealed),
        ] {
            self.inspect()?;
            let leaf = open_at(
                self.original.leaf()?,
                ".",
                libc::O_RDONLY | libc::O_DIRECTORY,
                0,
            )
            .map_err(StorageAttemptError::from)?;
            self.document
                .as_mut()
                .ok_or(StorageAttemptError::Refused)?
                .owned_write(&leaf, bytes, seal)?;
            self.boundary(at, &mut checkpoint)?;
        }
        self.document
            .as_ref()
            .ok_or(StorageAttemptError::Refused)?
            .file
            .sync_all()
            .map_err(StorageAttemptError::from)?;
        self.boundary(CollectorReservedStage::FileSynced, &mut checkpoint)?;
        self.original
            .leaf()?
            .sync_all()
            .map_err(StorageAttemptError::from)?;
        self.boundary(CollectorReservedStage::DirectorySynced, &mut checkpoint)?;
        self.boundary(CollectorReservedStage::Acknowledging, &mut checkpoint)?;
        self.acknowledged = true;
        Ok(())
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), StorageAttemptError> {
        if self.is_refused() || !self.acknowledged {
            return Err(StorageAttemptError::Refused);
        }
        let result = if self.starting.is_some() {
            self.inspect_starting()
        } else {
            self.inspect()
        };
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
