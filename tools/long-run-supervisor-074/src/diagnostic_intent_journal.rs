//! Fenced /tmp fixture consumption only; no backend or execution permission.
use super::{open_at, open_root, Directory, FixtureFence, PreparationError};
use crate::diagnostic_ipc::DiagnosticPeer;
use crate::diagnostic_lease::DiagnosticState;
use crate::diagnostic_worker_policy::intent_projection::{
    CheckedIntentProjection, IntentCheck, IntentProjectionError, SignedIntentProjection,
};
use crate::{canonical_json, sha256_hex};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::{Component, Path};
use thiserror::Error;

pub const RECORD_SCHEMA: &str = "diagnostic-retained-intent-074-v1";
pub const MAX_RECORD_BYTES: usize = 16_384;
pub const MAX_CELLS: usize = 4;
pub const MARKER_BYTES: usize = 65;
pub const MAX_DIRECTORY_ENTRIES: usize = 64;
const PREFIX: &str = "diagnostic-intent-cell-";

#[derive(Debug, Error)]
pub enum IntentJournalError {
    #[error("original signed intent refused: {0}")]
    Intent(#[from] IntentProjectionError),
    #[error("fixture host fence refused: {0}")]
    Fence(#[from] PreparationError),
    #[error("existing fixture lifecycle refused: {0}")]
    Lifecycle(#[from] super::recovery::FixtureRecoveryError),
    #[error("intent document, metadata, original object or fixture path is unsafe")]
    Document,
    #[error("intent publication is partial or unacknowledged")]
    Uncertain,
    #[error("cell, request or nonce is already consumed")]
    Replay,
    #[error("intent history differs from original lease, boot, cell or revision")]
    History,
    #[error("intent journal I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Inspection/retention data only, with no execution or admission conversion.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::intent_journal::FixtureIntentObservation;
/// fn start(observation: FixtureIntentObservation) { observation.start_once(); }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureIntentObservation {
    pub consumed_cells: usize,
    pub last_record_sha256: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: String,
    state: DiagnosticState,
    signed_intent: String,
    peer_uid: u32,
    peer_gids: Vec<u32>,
    previous_record_sha256: Option<String>,
}

#[derive(PartialEq, Eq)]
struct FileStamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl FileStamp {
    fn of(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
            len: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}
struct Leaf {
    name: String,
    file: File,
    stamp: FileStamp,
    raw: Vec<u8>,
    mode: u32,
    budget: usize,
}
impl Leaf {
    fn read(
        fence: &FixtureFence,
        name: &str,
        mode: u32,
        budget: usize,
    ) -> Result<Option<Self>, IntentJournalError> {
        let file = match open_at(&fence.root.file, name, libc::O_RDONLY, 0) {
            Err(PreparationError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None)
            }
            result => result?,
        };
        let m = file.metadata()?;
        regular(&m, fence, mode)?;
        if m.len() > budget as u64 {
            return Err(IntentJournalError::Document);
        }
        let stamp = FileStamp::of(&m);
        let raw = read_bytes(&file, budget)?;
        if raw.len() as u64 != stamp.len || FileStamp::of(&file.metadata()?) != stamp {
            return Err(IntentJournalError::Document);
        }
        Ok(Some(Self {
            name: name.into(),
            file,
            stamp,
            raw,
            mode,
            budget,
        }))
    }
    fn revalidate(&self, fence: &FixtureFence) -> Result<(), IntentJournalError> {
        let named = Self::read(fence, &self.name, self.mode, self.budget)?
            .ok_or(IntentJournalError::Document)?;
        if named.stamp != self.stamp
            || named.raw != self.raw
            || FileStamp::of(&self.file.metadata()?) != self.stamp
            || read_bytes(&self.file, self.budget)? != self.raw
        {
            return Err(IntentJournalError::Document);
        }
        Ok(())
    }
}
fn read_bytes(file: &File, budget: usize) -> Result<Vec<u8>, IntentJournalError> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let n = file.read_at(&mut buffer, bytes.len() as u64)?;
        if n == 0 {
            return Ok(bytes);
        }
        if bytes.len() + n > budget {
            return Err(IntentJournalError::Document);
        }
        bytes.extend_from_slice(&buffer[..n]);
    }
}
fn regular(m: &Metadata, fence: &FixtureFence, mode: u32) -> Result<(), IntentJournalError> {
    if !m.is_file()
        || m.nlink() != 1
        || m.mode() & 0o7777 != mode
        || m.uid() != fence.uid
        || m.gid() != fence.gid
        || m.dev() != fence.root.file.metadata()?.dev()
    {
        return Err(IntentJournalError::Document);
    }
    Ok(())
}
struct Capture {
    state: Leaf,
    ledger: Option<Leaf>,
    parent: Directory,
    parent_name: String,
}
impl Capture {
    fn open(fence: &FixtureFence, check: &IntentCheck<'_>) -> Result<Self, IntentJournalError> {
        let state = Leaf::read(fence, "active-diagnostic.json", 0o600, 65_536)?
            .ok_or(IntentJournalError::Document)?;
        let ledger = Leaf::read(
            fence,
            "diagnostic-requests-v1.json",
            0o600,
            crate::diagnostic_ipc::MAX_LEDGER_BYTES as usize,
        )?;
        let parent_path = fence.path.parent().ok_or(IntentJournalError::Document)?;
        let parent_name = parent_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(IntentJournalError::Document)?
            .into();
        let parent = Directory::checked(open_root(parent_path)?, fence.uid, fence.gid)?;
        let capture = Self {
            state,
            ledger,
            parent,
            parent_name,
        };
        if crate::diagnostic_lease::decode_active(&capture.state.raw)
            .map_err(|_| IntentJournalError::Document)?
            != *check.state
        {
            return Err(IntentJournalError::History);
        }
        capture.check(fence, check)?;
        Ok(capture)
    }
    fn check(
        &self,
        fence: &FixtureFence,
        check: &IntentCheck<'_>,
    ) -> Result<(), IntentJournalError> {
        fence.revalidate()?;
        self.parent
            .revalidate_at(&open_root(Path::new("/tmp"))?, &self.parent_name)?;
        self.state.revalidate(fence)?;
        match &self.ledger {
            Some(ledger) => ledger.revalidate(fence)?,
            None => {
                if Leaf::read(
                    fence,
                    "diagnostic-requests-v1.json",
                    0o600,
                    crate::diagnostic_ipc::MAX_LEDGER_BYTES as usize,
                )?
                .is_some()
                {
                    return Err(IntentJournalError::Document);
                }
            }
        }
        super::recovery::observe(fence, &check.state.identity, |_| {})?;
        fence.revalidate()?;
        Ok(())
    }
}
struct Entry {
    record: Leaf,
    marker: Leaf,
    decoded: Record,
}
impl Entry {
    fn revalidate(&self, fence: &FixtureFence) -> Result<(), IntentJournalError> {
        self.record.revalidate(fence)?;
        self.marker.revalidate(fence)
    }
}
fn name(cell: usize, suffix: &str) -> String {
    format!("{PREFIX}{}.{suffix}", cell + 1)
}
fn inventory(fence: &FixtureFence) -> Result<usize, IntentJournalError> {
    let mut total = 0;
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(format!("/proc/self/fd/{}", fence.root.file.as_raw_fd()))? {
        total += 1;
        if total > MAX_DIRECTORY_ENTRIES {
            return Err(IntentJournalError::Document);
        }
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| IntentJournalError::Document)?;
        if name.starts_with("diagnostic-intent-") || name.starts_with(".diagnostic-intent-") {
            seen.insert(name);
        }
    }
    if seen.len() % 2 != 0 {
        return Err(IntentJournalError::Uncertain);
    }
    let cells = seen.len() / 2;
    if cells > MAX_CELLS
        || seen
            != (0..cells)
                .flat_map(|i| [name(i, "json"), name(i, "commit")])
                .collect()
    {
        return Err(IntentJournalError::History);
    }
    Ok(cells)
}
fn history(
    fence: &FixtureFence,
    guard: &mut CheckedIntentProjection<'_>,
    check: &IntentCheck<'_>,
) -> Result<Vec<Entry>, IntentJournalError> {
    let cells = inventory(fence)?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut requests = BTreeSet::new();
    let mut nonces = BTreeSet::new();
    for cell in 0..cells {
        let record = Leaf::read(fence, &name(cell, "json"), 0o400, MAX_RECORD_BYTES)?
            .ok_or(IntentJournalError::Uncertain)?;
        let marker = Leaf::read(fence, &name(cell, "commit"), 0o400, MARKER_BYTES)?
            .ok_or(IntentJournalError::Uncertain)?;
        if marker.raw != marker_bytes(&record.raw) {
            return Err(IntentJournalError::Uncertain);
        }
        let decoded: Record =
            serde_json::from_slice(&record.raw).map_err(|_| IntentJournalError::Document)?;
        if decoded.schema_version != RECORD_SCHEMA
            || record_bytes(&decoded)? != record.raw
            || decoded.state.completed_cells != cell
            || decoded.state.identity != check.state.identity
            || decoded.state.revision > check.state.revision
            || decoded.state.last_observed_monotonic_ns > check.state.last_observed_monotonic_ns
            || decoded.state.controller_monotonic_ns > check.state.controller_monotonic_ns
            || decoded.state.reserved_monotonic_ns != check.state.reserved_monotonic_ns
            || decoded.previous_record_sha256 != entries.last().map(|e| sha256_hex(&e.record.raw))
        {
            return Err(IntentJournalError::History);
        }
        if let Some(previous) = entries.last() {
            if decoded.state.revision <= previous.decoded.state.revision
                || decoded.state.last_observed_monotonic_ns
                    < previous.decoded.state.last_observed_monotonic_ns
                || decoded.state.controller_monotonic_ns
                    < previous.decoded.state.controller_monotonic_ns
            {
                return Err(IntentJournalError::History);
            }
        }
        let body = guard.verify_consumed(
            decoded.signed_intent.as_bytes(),
            &decoded.state,
            &DiagnosticPeer {
                uid: decoded.peer_uid,
                gids: decoded.peer_gids.clone(),
            },
            check,
        )?;
        if !requests.insert(body.request_id) || !nonces.insert(body.nonce_sha256) {
            return Err(IntentJournalError::Replay);
        }
        entries.push(Entry {
            record,
            marker,
            decoded,
        });
    }
    for entry in &entries {
        entry.revalidate(fence)?;
    }
    if inventory(fence)? != cells {
        return Err(IntentJournalError::History);
    }
    Ok(entries)
}
fn record_bytes(record: &Record) -> Result<Vec<u8>, IntentJournalError> {
    let mut bytes = canonical_json(record).map_err(|_| IntentJournalError::Document)?;
    bytes.push(b'\n');
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(IntentJournalError::Document);
    }
    Ok(bytes)
}
fn marker_bytes(record: &[u8]) -> Vec<u8> {
    format!("{}\n", sha256_hex(record)).into_bytes()
}
fn fixture_path(root: &Path) -> Result<(), IntentJournalError> {
    let mut parts = root.components();
    if parts.next() != Some(Component::RootDir)
        || parts.next() != Some(Component::Normal("tmp".as_ref()))
    {
        return Err(IntentJournalError::Document);
    }
    let Some(Component::Normal(temp)) = parts.next() else {
        return Err(IntentJournalError::Document);
    };
    let temp = temp.to_str().ok_or(IntentJournalError::Document)?;
    if !temp.starts_with("hydracache-intent-")
        || temp.len() <= 18
        || temp.len() > 128
        || !temp
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || parts.next() != Some(Component::Normal("campaigns".as_ref()))
        || parts.next().is_some()
    {
        return Err(IntentJournalError::Document);
    }
    Ok(())
}
fn view(entries: &[Entry]) -> FixtureIntentObservation {
    FixtureIntentObservation {
        consumed_cells: entries.len(),
        last_record_sha256: entries.last().map(|e| sha256_hex(&e.record.raw)),
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PublicationStage {
    RecordCreated,
    RecordSynced,
    MarkerCreated,
    DurableCommit,
}
fn seal(
    file: &mut File,
    bytes: &[u8],
    fence: &FixtureFence,
) -> Result<FileStamp, IntentJournalError> {
    regular(&file.metadata()?, fence, 0o600)?;
    if file.metadata()?.len() != 0 {
        return Err(IntentJournalError::Document);
    }
    file.write_all(bytes)?;
    file.set_permissions(fs::Permissions::from_mode(0o400))?;
    file.sync_all()?;
    regular(&file.metadata()?, fence, 0o400)?;
    Ok(FileStamp::of(&file.metadata()?))
}
fn created_matches(
    fence: &FixtureFence,
    file: &File,
    stamp: &FileStamp,
    name: &str,
    raw: &[u8],
    budget: usize,
) -> Result<(), IntentJournalError> {
    let named = Leaf::read(fence, name, 0o400, budget)?.ok_or(IntentJournalError::Document)?;
    if FileStamp::of(&file.metadata()?) != *stamp || named.stamp != *stamp || named.raw != raw {
        return Err(IntentJournalError::Document);
    }
    Ok(())
}
fn operate(
    root: &Path,
    guard: &mut CheckedIntentProjection<'_>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
    retain: bool,
    mut checkpoint: impl FnMut(PublicationStage, &FixtureFence) -> Result<(), IntentJournalError>,
) -> Result<FixtureIntentObservation, IntentJournalError> {
    let result = (|| {
        guard.revalidate(bytes, check)?;
        fixture_path(root)?;
        let fence = FixtureFence::acquire(root)?;
        let capture = Capture::open(&fence, check)?;
        let entries = history(&fence, guard, check)?;
        if !retain {
            let result = view(&entries);
            capture.check(&fence, check)?;
            fence.release()?;
            return Ok(result);
        }
        if entries.len() > check.state.completed_cells {
            return Err(IntentJournalError::Replay);
        }
        if entries.len() != check.state.completed_cells || entries.len() == MAX_CELLS {
            return Err(IntentJournalError::History);
        }
        let current: SignedIntentProjection =
            serde_json::from_slice(bytes).map_err(|_| IntentJournalError::Document)?;
        for entry in &entries {
            let prior: SignedIntentProjection = serde_json::from_str(&entry.decoded.signed_intent)
                .map_err(|_| IntentJournalError::Document)?;
            if prior.intent.request_id == current.intent.request_id
                || prior.intent.nonce_sha256 == current.intent.nonce_sha256
            {
                return Err(IntentJournalError::Replay);
            }
        }
        let record = Record {
            schema_version: RECORD_SCHEMA.into(),
            state: check.state.clone(),
            signed_intent: std::str::from_utf8(bytes)
                .map_err(|_| IntentJournalError::Document)?
                .into(),
            peer_uid: check.peer.uid,
            peer_gids: check.peer.gids.clone(),
            previous_record_sha256: entries.last().map(|e| sha256_hex(&e.record.raw)),
        };
        let raw = record_bytes(&record)?;
        let cell = entries.len();
        let validate = || -> Result<(), IntentJournalError> {
            capture.check(&fence, check)?;
            for entry in &entries {
                entry.revalidate(&fence)?;
            }
            Ok(())
        };
        validate()?;
        // Persist the already owned fixture ancestry before recording consumption.
        capture.parent.file.sync_all()?;
        open_root(Path::new("/tmp"))?.sync_all()?;
        let mut file = open_at(
            &fence.root.file,
            &name(cell, "json"),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        fence.root.file.sync_all()?;
        checkpoint(PublicationStage::RecordCreated, &fence)?;
        validate()?;
        let record_stamp = seal(&mut file, &raw, &fence)?;
        let validate_created = || -> Result<(), IntentJournalError> {
            validate()?;
            created_matches(
                &fence,
                &file,
                &record_stamp,
                &name(cell, "json"),
                &raw,
                MAX_RECORD_BYTES,
            )
        };
        checkpoint(PublicationStage::RecordSynced, &fence)?;
        validate_created()?;
        let mut marker = open_at(
            &fence.root.file,
            &name(cell, "commit"),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
            0o600,
        )?;
        fence.root.file.sync_all()?;
        checkpoint(PublicationStage::MarkerCreated, &fence)?;
        validate_created()?;
        let marker_raw = marker_bytes(&raw);
        let marker_stamp = seal(&mut marker, &marker_raw, &fence)?;
        fence.root.file.sync_all()?;
        checkpoint(PublicationStage::DurableCommit, &fence)?;
        validate_created()?;
        created_matches(
            &fence,
            &marker,
            &marker_stamp,
            &name(cell, "commit"),
            &marker_raw,
            MARKER_BYTES,
        )?;
        let committed = history(&fence, guard, check)?;
        if committed.len() != cell + 1
            || committed
                .last()
                .ok_or(IntentJournalError::History)?
                .record
                .raw
                != raw
        {
            return Err(IntentJournalError::History);
        }
        let result = view(&committed);
        capture.check(&fence, check)?;
        validate_created()?;
        created_matches(
            &fence,
            &marker,
            &marker_stamp,
            &name(cell, "commit"),
            &marker_raw,
            MARKER_BYTES,
        )?;
        fence.release()?;
        Ok(result)
    })();
    if result.is_err() {
        guard.latch_refusal();
    }
    result
}
/// Retain one exact signed cell before any future execution. Result is not a permit.
pub fn retain_fixture_intent(
    root: &Path,
    guard: &mut CheckedIntentProjection<'_>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<FixtureIntentObservation, IntentJournalError> {
    operate(root, guard, bytes, check, true, |_, _| Ok(()))
}
/// Inspect consumed cells only; never replay, repair or adopt an execution.
pub fn inspect_fixture_intents(
    root: &Path,
    guard: &mut CheckedIntentProjection<'_>,
    bytes: &[u8],
    check: &IntentCheck<'_>,
) -> Result<FixtureIntentObservation, IntentJournalError> {
    operate(root, guard, bytes, check, false, |_, _| Ok(()))
}

#[cfg(test)]
#[path = "diagnostic_intent_journal_tests.rs"]
mod tests;
