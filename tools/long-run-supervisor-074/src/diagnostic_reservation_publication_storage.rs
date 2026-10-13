//! Private original-object publication engine; no authority or execution entry.
use super::super::super::super::open_at;
use super::super::super::{Document, FixedReservedStateError, Stamp, MAX_BYTES};
use super::attempt_storage::{
    Document as AttemptDocument, StorageAttemptError, NAME as ATTEMPT_NAME,
};
use super::{
    parent as performance_parent, EmptyReservationFenceError, OriginalEmptyReservationFence,
};
use crate::diagnostic_lease::{DiagnosticState, ACTIVE_DIAGNOSTIC_NAME};
use std::ffi::CStr;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsRawFd, IntoRawFd};
use thiserror::Error;

#[path = "diagnostic_initial_consumption_storage.rs"]
pub(crate) mod initial_consumption_storage;
use initial_consumption_storage::ConsumptionDocument;
pub use initial_consumption_storage::ConsumptionStage;
#[path = "diagnostic_starting_intent_storage.rs"]
mod starting_intent_storage;
pub use starting_intent_storage::StartingStage;
use starting_intent_storage::NAME as STARTING_NAME;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationStage {
    BeforeCreate,
    StateCreated,
    PrefixWritten,
    BytesWritten,
    FileSynced,
    DirectorySynced,
    DocumentBound,
    Acknowledging,
}
#[derive(Debug, Error)]
pub enum StoragePublicationError {
    #[error("original inherited attempt refused: {0}")]
    Attempt(#[from] StorageAttemptError),
    #[error("original empty storage/fence refused: {0}")]
    Fence(#[from] EmptyReservationFenceError),
    #[error("original created state document refused: {0}")]
    State(#[from] FixedReservedStateError),
    #[error("publication has other retained fixed-leaf context")]
    Context,
    #[error("publication already attempted or original storage refused")]
    Refused,
    #[error("exclusive created document open failed: {0}")]
    Open(#[from] crate::diagnostic_output_preparation::PreparationError),
    #[error("state publication I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("local injected interruption at {0:?}")]
    Interrupted(PublicationStage),
    #[error("initial retained intent encoding refused: {0}")]
    Record(#[from] crate::diagnostic_output_preparation::intent_journal::IntentJournalError),
    #[error("original consumed intent object refused")]
    ConsumptionDocument,
    #[error("local injected consumption interruption at {0:?}")]
    ConsumptionInterrupted(ConsumptionStage),
    #[error("local injected Starting intent interruption at {0:?}")]
    StartingInterrupted(StartingStage),
}
pub(crate) struct PublicationStorage<'storage> {
    pub(crate) empty: OriginalEmptyReservationFence<'storage>,
    attempt: Option<AttemptDocument>,
    document: Option<Document>,
    attempted: bool,
    acknowledged: bool,
    consumption_started: bool,
    consumption_acknowledged: bool,
    consumption_documents: [Option<ConsumptionDocument>; 2],
    starting_document: Option<ConsumptionDocument>,
    starting_attempted: bool,
    starting_acknowledged: bool,
}
impl<'storage> PublicationStorage<'storage> {
    pub(crate) fn new(empty: OriginalEmptyReservationFence<'storage>) -> Self {
        Self {
            empty,
            attempt: None,
            document: None,
            attempted: false,
            acknowledged: false,
            consumption_started: false,
            consumption_acknowledged: false,
            consumption_documents: [None, None],
            starting_document: None,
            starting_attempted: false,
            starting_acknowledged: false,
        }
    }
    pub(super) fn from_attempt(
        empty: OriginalEmptyReservationFence<'storage>,
        attempt: AttemptDocument,
    ) -> Self {
        let mut storage = Self::new(empty);
        storage.attempt = Some(attempt);
        storage
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.empty.is_refused()
    }
    pub(crate) fn latch_refusal(&mut self) {
        self.empty.latch_refusal();
    }
    fn leaf(&self) -> Result<&File, StoragePublicationError> {
        Ok(&self
            .empty
            .storage
            .components
            .last()
            .ok_or(EmptyReservationFenceError::Origin)?
            .file)
    }
    fn ancestry(&mut self) -> Result<(), StoragePublicationError> {
        self.empty
            .storage
            .revalidate()
            .map_err(EmptyReservationFenceError::from)?;
        self.empty
            .fence
            .inspect(performance_parent(self.empty.storage)?)
            .map_err(EmptyReservationFenceError::from)?;
        Ok(())
    }
    fn inspect_owned_attempt(&self) -> Result<(), StoragePublicationError> {
        if let Some(attempt) = &self.attempt {
            attempt.inspect(self.leaf()?)?;
        }
        Ok(())
    }
    fn publication_namespace(&self, state_present: bool) -> Result<(), StoragePublicationError> {
        self.inspect_owned_attempt()?;
        if self.attempt.is_none() {
            if !state_present {
                return Err(StoragePublicationError::Context);
            }
            only_state(self.leaf()?)?;
            return Ok(());
        }
        let fresh = open_at(self.leaf()?, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let mut seen = [false; 2];
        for entry in std::fs::read_dir(format!("/proc/self/fd/{}", fresh.as_raw_fd()))? {
            let name = entry?.file_name();
            let index = if name == ATTEMPT_NAME {
                1
            } else if state_present && name == ACTIVE_DIAGNOSTIC_NAME {
                0
            } else {
                return Err(StoragePublicationError::Context);
            };
            if seen[index] {
                return Err(StoragePublicationError::Context);
            }
            seen[index] = true;
        }
        if seen != [state_present, true] {
            return Err(StoragePublicationError::Context);
        }
        self.inspect_owned_attempt()
    }
    fn before_publication(&mut self) -> Result<(), StoragePublicationError> {
        if self.attempt.is_none() {
            return self.empty.revalidate().map_err(Into::into);
        }
        self.ancestry()?;
        self.publication_namespace(false)?;
        self.ancestry()?;
        self.publication_namespace(false)
    }
    fn objects(&mut self, file: &File) -> Result<(), StoragePublicationError> {
        self.ancestry()?;
        let stamp = Stamp::of(&file.metadata()?);
        if !stamp.safe(&self.leaf()?.metadata()?) {
            return Err(FixedReservedStateError::Document.into());
        }
        let named = open_at(self.leaf()?, ACTIVE_DIAGNOSTIC_NAME, libc::O_RDONLY, 0)?;
        if Stamp::of(&named.metadata()?) != stamp || Stamp::of(&file.metadata()?) != stamp {
            return Err(FixedReservedStateError::Document.into());
        }
        self.publication_namespace(true)?;
        self.ancestry()
    }
    fn boundary<E: From<StoragePublicationError>>(
        &mut self,
        file: &File,
        stage: PublicationStage,
        checkpoint: &mut impl FnMut(PublicationStage) -> Result<(), E>,
    ) -> Result<(), E> {
        self.objects(file)?;
        checkpoint(stage)?;
        self.objects(file)?;
        Ok(())
    }
    pub(crate) fn publish_with<E: From<StoragePublicationError>>(
        &mut self,
        expected: &DiagnosticState,
        mut checkpoint: impl FnMut(PublicationStage) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.attempted || self.is_refused() {
            return Err(StoragePublicationError::Refused.into());
        }
        self.attempted = true;
        let raw = crate::diagnostic_lease::encode(expected)
            .map_err(|_| StoragePublicationError::State(FixedReservedStateError::Expected))?;
        if raw.len() as u64 > MAX_BYTES {
            return Err(StoragePublicationError::State(FixedReservedStateError::Document).into());
        }
        self.before_publication()?;
        checkpoint(PublicationStage::BeforeCreate)?;
        if self.attempt.is_some() {
            self.before_publication()?;
        }
        // Exclusion protects even a same-name appearance after the last empty scan.
        let mut file = open_at(
            self.leaf()?,
            ACTIVE_DIAGNOSTIC_NAME,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )
        .map_err(StoragePublicationError::from)?;
        self.boundary(&file, PublicationStage::StateCreated, &mut checkpoint)?;
        let middle = raw.len() / 2;
        file.write_all(&raw[..middle])
            .map_err(StoragePublicationError::from)?;
        self.boundary(&file, PublicationStage::PrefixWritten, &mut checkpoint)?;
        file.write_all(&raw[middle..])
            .map_err(StoragePublicationError::from)?;
        self.boundary(&file, PublicationStage::BytesWritten, &mut checkpoint)?;
        file.sync_all().map_err(StoragePublicationError::from)?;
        self.boundary(&file, PublicationStage::FileSynced, &mut checkpoint)?;
        self.leaf()?
            .sync_all()
            .map_err(StoragePublicationError::from)?;
        self.boundary(&file, PublicationStage::DirectorySynced, &mut checkpoint)?;
        let document = Document {
            stamp: Stamp::of(&file.metadata().map_err(StoragePublicationError::from)?),
            file,
            raw,
        };
        document
            .revalidate(self.leaf()?)
            .map_err(StoragePublicationError::from)?;
        self.document = Some(document);
        self.inspect()?;
        checkpoint(PublicationStage::DocumentBound)?;
        self.inspect()?;
        checkpoint(PublicationStage::Acknowledging)?;
        self.inspect()?;
        self.acknowledged = true;
        Ok(())
    }
    fn inspect(&mut self) -> Result<(), StoragePublicationError> {
        self.ancestry()?;
        let doc = self
            .document
            .as_ref()
            .ok_or(StoragePublicationError::Refused)?;
        doc.revalidate(self.leaf()?)?;
        self.inspect_consumption_namespace()?;
        self.ancestry()?;
        self.document
            .as_ref()
            .ok_or(StoragePublicationError::Refused)?
            .revalidate(self.leaf()?)?;
        self.inspect_consumption_namespace()?;
        self.ancestry()
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), StoragePublicationError> {
        if self.is_refused() || !self.acknowledged {
            return Err(StoragePublicationError::Refused);
        }
        let result = self.inspect();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
struct DirectoryStream(*mut libc::DIR);
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}
fn only_state(leaf: &File) -> Result<(), StoragePublicationError> {
    let fresh = open_at(leaf, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    let fd = fresh.into_raw_fd();
    let pointer = unsafe { libc::fdopendir(fd) };
    if pointer.is_null() {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::close(fd);
        }
        return Err(error.into());
    }
    let stream = DirectoryStream(pointer);
    let mut seen = false;
    loop {
        unsafe {
            *libc::__errno_location() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            let errno = unsafe { *libc::__errno_location() };
            return if errno != 0 {
                Err(std::io::Error::from_raw_os_error(errno).into())
            } else if seen {
                Ok(())
            } else {
                Err(StoragePublicationError::Context)
            };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if seen || name != ACTIVE_DIAGNOSTIC_NAME.as_bytes() {
            return Err(StoragePublicationError::Context);
        }
        seen = true;
    }
}
