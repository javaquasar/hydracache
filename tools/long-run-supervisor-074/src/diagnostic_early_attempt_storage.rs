//! One original early-attempt object; no state advancement or recovered adoption.
use super::super::super::super::open_at;
use super::{parent, EmptyReservationFenceError, OriginalEmptyReservationFence};
use crate::diagnostic_output_preparation::{
    intent_journal::{IntentJournalError, MAX_RECORD_BYTES},
    PreparationError,
};
use std::fs::{File, Metadata};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt};
use thiserror::Error;

const NAME: &str = "diagnostic-attempt-074.json";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptStage {
    BeforeCreate,
    Created,
    PrefixWritten,
    BytesWritten,
    Sealed,
    FileSynced,
    DirectorySynced,
    Acknowledging,
}
#[derive(Debug, Error)]
pub enum StorageAttemptError {
    #[error("original empty fence refused: {0}")]
    Fence(#[from] EmptyReservationFenceError),
    #[error("attempt has unexpected retained context")]
    Context,
    #[error("original attempt document changed or is unsafe")]
    Document,
    #[error("original attempt already used or refused")]
    Refused,
    #[error("attempt open refused: {0}")]
    Open(#[from] PreparationError),
    #[error("attempt encoding refused: {0}")]
    Record(#[from] IntentJournalError),
    #[error("attempt I/O refused: {0}")]
    Io(#[from] std::io::Error),
    #[error("local interruption at {0:?}")]
    Interrupted(AttemptStage),
}
fn stamp(m: &Metadata) -> [u64; 11] {
    [
        m.dev(),
        m.ino(),
        u64::from(m.uid()),
        u64::from(m.gid()),
        u64::from(m.mode()),
        m.nlink(),
        m.len(),
        m.mtime() as u64,
        m.mtime_nsec() as u64,
        m.ctime() as u64,
        m.ctime_nsec() as u64,
    ]
}
struct Document {
    file: File,
    stamp: [u64; 11],
    raw: Vec<u8>,
    mode: u32,
}
impl Document {
    fn safe(&self, leaf: &File, m: &Metadata) -> Result<(), StorageAttemptError> {
        let p = leaf.metadata()?;
        if !m.is_file()
            || m.nlink() != 1
            || m.mode() & 0o7777 != self.mode
            || m.uid() != p.uid()
            || m.gid() != p.gid()
            || m.dev() != p.dev()
            || m.len() > MAX_RECORD_BYTES as u64
            || stamp(m)[..2] != self.stamp[..2]
        {
            return Err(StorageAttemptError::Document);
        }
        Ok(())
    }
    fn bytes(&self, file: &File) -> Result<(), StorageAttemptError> {
        let mut bytes = vec![0; self.raw.len() + 1];
        let mut used = 0;
        loop {
            let count = file.read_at(&mut bytes[used..], used as u64)?;
            if count == 0 {
                break;
            }
            used += count;
            if used == bytes.len() {
                return Err(StorageAttemptError::Document);
            }
        }
        if bytes[..used] != self.raw {
            return Err(StorageAttemptError::Document);
        }
        Ok(())
    }
    fn inspect(&self, leaf: &File) -> Result<(), StorageAttemptError> {
        let m = self.file.metadata()?;
        self.safe(leaf, &m)?;
        if stamp(&m) != self.stamp {
            return Err(StorageAttemptError::Document);
        }
        let named = open_at(leaf, NAME, libc::O_RDONLY, 0)?;
        if stamp(&named.metadata()?) != self.stamp {
            return Err(StorageAttemptError::Document);
        }
        self.bytes(&self.file)?;
        self.bytes(&named)?;
        if stamp(&self.file.metadata()?) != self.stamp || stamp(&named.metadata()?) != self.stamp {
            return Err(StorageAttemptError::Document);
        }
        Ok(())
    }
    fn owned_write(
        &mut self,
        leaf: &File,
        bytes: &[u8],
        seal: bool,
    ) -> Result<(), StorageAttemptError> {
        if self.raw.len() + bytes.len() > MAX_RECORD_BYTES || self.mode != 0o600 {
            return Err(StorageAttemptError::Document);
        }
        if seal {
            if !bytes.is_empty() {
                return Err(StorageAttemptError::Document);
            }
            if unsafe { libc::fchmod(self.file.as_raw_fd(), 0o400) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            self.mode = 0o400;
        } else {
            self.file.write_all(bytes)?;
            self.raw.extend_from_slice(bytes);
        }
        let m = self.file.metadata()?;
        self.safe(leaf, &m)?;
        self.stamp = stamp(&m);
        self.inspect(leaf)
    }
}
pub(crate) struct AttemptStorage<'storage> {
    pub(crate) empty: OriginalEmptyReservationFence<'storage>,
    document: Option<Document>,
    attempted: bool,
    acknowledged: bool,
}
impl<'storage> AttemptStorage<'storage> {
    pub(crate) fn new(empty: OriginalEmptyReservationFence<'storage>) -> Self {
        Self {
            empty,
            document: None,
            attempted: false,
            acknowledged: false,
        }
    }
    pub(crate) fn is_refused(&self) -> bool {
        self.empty.is_refused()
    }
    pub(crate) fn latch_refusal(&mut self) {
        self.empty.latch_refusal();
    }
    fn leaf(&self) -> Result<&File, StorageAttemptError> {
        Ok(&self
            .empty
            .storage
            .components
            .last()
            .ok_or(EmptyReservationFenceError::Origin)?
            .file)
    }
    fn ancestry(&mut self) -> Result<(), StorageAttemptError> {
        self.empty
            .storage
            .revalidate()
            .map_err(EmptyReservationFenceError::from)?;
        self.empty
            .fence
            .inspect(parent(self.empty.storage)?)
            .map_err(EmptyReservationFenceError::from)?;
        Ok(())
    }
    fn inspect(&mut self) -> Result<(), StorageAttemptError> {
        self.ancestry()?;
        self.document
            .as_ref()
            .ok_or(StorageAttemptError::Refused)?
            .inspect(self.leaf()?)?;
        let fresh = open_at(self.leaf()?, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let mut seen = false;
        for entry in std::fs::read_dir(format!("/proc/self/fd/{}", fresh.as_raw_fd()))? {
            if seen || entry?.file_name() != NAME {
                return Err(StorageAttemptError::Context);
            }
            seen = true;
        }
        if !seen {
            return Err(StorageAttemptError::Context);
        }
        self.ancestry()?;
        self.document
            .as_ref()
            .ok_or(StorageAttemptError::Refused)?
            .inspect(self.leaf()?)?;
        Ok(())
    }
    fn boundary<E: From<StorageAttemptError>>(
        &mut self,
        at: AttemptStage,
        checkpoint: &mut impl FnMut(AttemptStage) -> Result<(), E>,
    ) -> Result<(), E> {
        self.inspect()?;
        checkpoint(at)?;
        self.inspect()?;
        Ok(())
    }
    pub(crate) fn retain_with<E: From<StorageAttemptError>>(
        &mut self,
        raw: &[u8],
        mut checkpoint: impl FnMut(AttemptStage) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.is_refused() || self.attempted {
            return Err(StorageAttemptError::Refused.into());
        }
        self.attempted = true;
        if raw.is_empty() || raw.len() > MAX_RECORD_BYTES {
            return Err(StorageAttemptError::Document.into());
        }
        self.empty.revalidate().map_err(StorageAttemptError::from)?;
        checkpoint(AttemptStage::BeforeCreate)?;
        self.empty.revalidate().map_err(StorageAttemptError::from)?;
        let file = open_at(
            self.leaf()?,
            NAME,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )
        .map_err(StorageAttemptError::from)?;
        self.document = Some(Document {
            stamp: stamp(&file.metadata().map_err(StorageAttemptError::from)?),
            file,
            raw: Vec::new(),
            mode: 0o600,
        });
        self.inspect()?;
        self.leaf()?.sync_all().map_err(StorageAttemptError::from)?;
        self.boundary(AttemptStage::Created, &mut checkpoint)?;
        let middle = raw.len() / 2;
        for (bytes, seal, stage) in [
            (&raw[..middle], false, AttemptStage::PrefixWritten),
            (&raw[middle..], false, AttemptStage::BytesWritten),
            (&[][..], true, AttemptStage::Sealed),
        ] {
            self.inspect()?;
            let leaf = open_at(self.leaf()?, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)
                .map_err(StorageAttemptError::from)?;
            self.document
                .as_mut()
                .ok_or(StorageAttemptError::Refused)?
                .owned_write(&leaf, bytes, seal)?;
            self.boundary(stage, &mut checkpoint)?;
        }
        self.document
            .as_ref()
            .ok_or(StorageAttemptError::Refused)?
            .file
            .sync_all()
            .map_err(StorageAttemptError::from)?;
        self.boundary(AttemptStage::FileSynced, &mut checkpoint)?;
        self.leaf()?.sync_all().map_err(StorageAttemptError::from)?;
        self.boundary(AttemptStage::DirectorySynced, &mut checkpoint)?;
        self.boundary(AttemptStage::Acknowledging, &mut checkpoint)?;
        self.acknowledged = true;
        Ok(())
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), StorageAttemptError> {
        if self.is_refused() || !self.acknowledged {
            return Err(StorageAttemptError::Refused);
        }
        let result = self.inspect();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
