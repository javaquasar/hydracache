//! Private exact two-object consumption; never reopens a runtime acknowledgement.
use super::{
    open_at, DirectoryStream, PublicationStorage, StoragePublicationError, ATTEMPT_NAME,
    STARTING_NAME,
};
use crate::diagnostic_lease::ACTIVE_DIAGNOSTIC_NAME;
use crate::diagnostic_output_preparation::intent_journal::{
    marker_bytes, MARKER_BYTES, MAX_RECORD_BYTES,
};
use std::ffi::CStr;
use std::fs::{File, Metadata};
use std::io::Write;
use std::os::fd::{AsRawFd, IntoRawFd};
use std::os::unix::fs::{FileExt, MetadataExt};

const NAMES: [&str; 2] = [
    "diagnostic-intent-cell-1.json",
    "diagnostic-intent-cell-1.commit",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsumptionStage {
    BeforeRecord,
    RecordCreated,
    RecordPrefixWritten,
    RecordBytesWritten,
    RecordSealed,
    RecordSynced,
    BeforeCommit,
    CommitCreated,
    CommitPrefixWritten,
    CommitBytesWritten,
    CommitSealed,
    CommitSynced,
    DirectorySynced,
    Acknowledging,
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
pub(super) struct ConsumptionDocument {
    file: File,
    name: &'static str,
    raw: Vec<u8>,
    stamp: [u64; 11],
    mode: u32,
    budget: usize,
}
impl ConsumptionDocument {
    fn safe(&self, parent: &File, m: &Metadata) -> Result<(), StoragePublicationError> {
        let p = parent.metadata()?;
        if !m.is_file()
            || m.mode() & 0o7777 != self.mode
            || m.nlink() != 1
            || m.uid() != p.uid()
            || m.gid() != p.gid()
            || m.dev() != p.dev()
            || m.len() > self.budget as u64
            || stamp(m)[..2] != self.stamp[..2]
        {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        Ok(())
    }
    fn bytes_match(&self, file: &File) -> Result<(), StoragePublicationError> {
        let mut bytes = vec![0; self.raw.len() + 1];
        let mut used = 0;
        loop {
            let count = file.read_at(&mut bytes[used..], used as u64)?;
            if count == 0 {
                break;
            }
            used += count;
            if used == bytes.len() {
                return Err(StoragePublicationError::ConsumptionDocument);
            }
        }
        if bytes[..used] != self.raw {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        Ok(())
    }
    pub(super) fn inspect(&self, parent: &File) -> Result<(), StoragePublicationError> {
        let original = self.file.metadata()?;
        self.safe(parent, &original)?;
        if stamp(&original) != self.stamp {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        let named = open_at(parent, self.name, libc::O_RDONLY, 0)?;
        if stamp(&named.metadata()?) != self.stamp {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        self.bytes_match(&self.file)?;
        self.bytes_match(&named)?;
        if stamp(&named.metadata()?) != self.stamp || stamp(&self.file.metadata()?) != self.stamp {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        Ok(())
    }
    fn refresh_owned_write(&mut self, parent: &File) -> Result<(), StoragePublicationError> {
        let original = self.file.metadata()?;
        self.safe(parent, &original)?;
        self.stamp = stamp(&original);
        self.inspect(parent)
    }
    pub(super) fn create(
        parent: &File,
        name: &'static str,
        budget: usize,
    ) -> Result<Self, StoragePublicationError> {
        let file = open_at(
            parent,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        let document = Self {
            stamp: stamp(&file.metadata()?),
            file,
            name,
            raw: Vec::new(),
            mode: 0o600,
            budget,
        };
        Ok(document)
    }
    pub(super) fn append_owned(
        &mut self,
        parent: &File,
        bytes: &[u8],
        seal: bool,
    ) -> Result<(), StoragePublicationError> {
        self.inspect(parent)?;
        if self.mode != 0o600
            || self.raw.len() + bytes.len() > self.budget
            || (seal && !bytes.is_empty())
        {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        if seal {
            if unsafe { libc::fchmod(self.file.as_raw_fd(), 0o400) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            self.mode = 0o400;
        } else {
            self.file.write_all(bytes)?;
            self.raw.extend_from_slice(bytes);
        }
        self.refresh_owned_write(parent)
    }
    pub(super) fn sync_owned(&self, parent: &File) -> Result<(), StoragePublicationError> {
        self.inspect(parent)?;
        self.file.sync_all()?;
        self.inspect(parent)
    }
    pub(super) fn raw_sha256(&self) -> String {
        crate::sha256_hex(&self.raw)
    }
}
impl PublicationStorage<'_> {
    pub(super) fn inspect_consumption_namespace(&self) -> Result<(), StoragePublicationError> {
        if !self.consumption_started {
            return self.publication_namespace(true);
        }
        self.inspect_owned_attempt()?;
        if let Some(document) = &self.starting_document {
            document.inspect(self.leaf()?)?;
        }
        for document in self.consumption_documents.iter().flatten() {
            document.inspect(self.leaf()?)?;
        }
        let fresh = open_at(self.leaf()?, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
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
        let mut seen = [false; 4];
        let mut seen_starting = false;
        loop {
            unsafe {
                *libc::__errno_location() = 0;
            }
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
                let errno = unsafe { *libc::__errno_location() };
                if errno != 0 {
                    return Err(std::io::Error::from_raw_os_error(errno).into());
                }
                break;
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            if name == STARTING_NAME.as_bytes() && self.starting_document.is_some() {
                if seen_starting {
                    return Err(StoragePublicationError::Context);
                }
                seen_starting = true;
                continue;
            }
            let index = if name == ACTIVE_DIAGNOSTIC_NAME.as_bytes() {
                0
            } else if name == NAMES[0].as_bytes() && self.consumption_documents[0].is_some() {
                1
            } else if name == NAMES[1].as_bytes() && self.consumption_documents[1].is_some() {
                2
            } else if name == ATTEMPT_NAME.as_bytes() && self.attempt.is_some() {
                3
            } else {
                return Err(StoragePublicationError::Context);
            };
            if seen[index] {
                return Err(StoragePublicationError::Context);
            }
            seen[index] = true;
        }
        if seen
            != [
                true,
                self.consumption_documents[0].is_some(),
                self.consumption_documents[1].is_some(),
                self.attempt.is_some(),
            ]
        {
            return Err(StoragePublicationError::Context);
        }
        if seen_starting != self.starting_document.is_some() {
            return Err(StoragePublicationError::Context);
        }
        if let Some(document) = &self.starting_document {
            document.inspect(self.leaf()?)?;
        }
        self.inspect_owned_attempt()
    }
    fn consumption_boundary<E: From<StoragePublicationError>>(
        &mut self,
        at: ConsumptionStage,
        checkpoint: &mut impl FnMut(ConsumptionStage) -> Result<(), E>,
    ) -> Result<(), E> {
        self.inspect()?;
        checkpoint(at)?;
        self.inspect()?;
        Ok(())
    }
    fn create_consumption_document(&mut self, index: usize) -> Result<(), StoragePublicationError> {
        let file = open_at(
            self.leaf()?,
            NAMES[index],
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        self.consumption_documents[index] = Some(ConsumptionDocument {
            stamp: stamp(&file.metadata()?),
            file,
            name: NAMES[index],
            raw: Vec::new(),
            mode: 0o600,
            budget: if index == 0 {
                MAX_RECORD_BYTES
            } else {
                MARKER_BYTES
            },
        });
        self.inspect()?;
        self.leaf()?.sync_all()?;
        Ok(())
    }
    fn write_consumption_document(
        &mut self,
        index: usize,
        bytes: &[u8],
        seal: bool,
    ) -> Result<(), StoragePublicationError> {
        self.inspect()?;
        let parent = open_at(self.leaf()?, ".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let document = self.consumption_documents[index]
            .as_mut()
            .ok_or(StoragePublicationError::Refused)?;
        if document.raw.len() + bytes.len() > document.budget {
            return Err(StoragePublicationError::ConsumptionDocument);
        }
        if seal {
            if !bytes.is_empty() {
                return Err(StoragePublicationError::ConsumptionDocument);
            }
            if unsafe { libc::fchmod(document.file.as_raw_fd(), 0o400) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            document.mode = 0o400;
        } else {
            document.file.write_all(bytes)?;
            document.raw.extend_from_slice(bytes);
        }
        document.refresh_owned_write(&parent)?;
        self.inspect()
    }
    pub(crate) fn consume_initial_with<E: From<StoragePublicationError>>(
        &mut self,
        raw: &[u8],
        mut checkpoint: impl FnMut(ConsumptionStage) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.is_refused() || !self.acknowledged || self.consumption_started {
            return Err(StoragePublicationError::Refused.into());
        }
        self.revalidate()?;
        if raw.is_empty() || raw.len() > MAX_RECORD_BYTES {
            return Err(StoragePublicationError::ConsumptionDocument.into());
        }
        self.consumption_started = true;
        self.consumption_boundary(ConsumptionStage::BeforeRecord, &mut checkpoint)?;
        let marker = marker_bytes(raw);
        for (index, bytes, stages) in [
            (
                0,
                raw,
                [
                    ConsumptionStage::RecordCreated,
                    ConsumptionStage::RecordPrefixWritten,
                    ConsumptionStage::RecordBytesWritten,
                    ConsumptionStage::RecordSealed,
                    ConsumptionStage::RecordSynced,
                ],
            ),
            (
                1,
                marker.as_slice(),
                [
                    ConsumptionStage::CommitCreated,
                    ConsumptionStage::CommitPrefixWritten,
                    ConsumptionStage::CommitBytesWritten,
                    ConsumptionStage::CommitSealed,
                    ConsumptionStage::CommitSynced,
                ],
            ),
        ] {
            self.create_consumption_document(index)?;
            self.consumption_boundary(stages[0], &mut checkpoint)?;
            let middle = bytes.len() / 2;
            self.write_consumption_document(index, &bytes[..middle], false)?;
            self.consumption_boundary(stages[1], &mut checkpoint)?;
            self.write_consumption_document(index, &bytes[middle..], false)?;
            self.consumption_boundary(stages[2], &mut checkpoint)?;
            self.write_consumption_document(index, &[], true)?;
            self.consumption_boundary(stages[3], &mut checkpoint)?;
            self.consumption_documents[index]
                .as_ref()
                .ok_or(StoragePublicationError::Refused)?
                .file
                .sync_all()
                .map_err(StoragePublicationError::from)?;
            self.consumption_boundary(stages[4], &mut checkpoint)?;
            if index == 0 {
                self.consumption_boundary(ConsumptionStage::BeforeCommit, &mut checkpoint)?;
            }
        }
        self.leaf()?
            .sync_all()
            .map_err(StoragePublicationError::from)?;
        self.consumption_boundary(ConsumptionStage::DirectorySynced, &mut checkpoint)?;
        self.consumption_boundary(ConsumptionStage::Acknowledging, &mut checkpoint)?;
        self.consumption_acknowledged = true;
        Ok(())
    }
    pub(crate) fn revalidate_initial_consumption(&mut self) -> Result<(), StoragePublicationError> {
        if !self.consumption_acknowledged {
            return Err(StoragePublicationError::Refused);
        }
        self.revalidate()
    }
}
