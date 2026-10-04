use crate::sealed_permissions::{make_tree_read_only, verify_tree_read_only};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tar::{Builder, EntryType, Header};
use thiserror::Error;

pub const ARCHIVE_NAME: &str = "packet.tar.zst";
pub const OUTER_DIGEST_NAME: &str = "outer-sha256.txt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ArchiveLimits {
    pub maximum_files: usize,
    pub maximum_uncompressed_bytes: u64,
    pub maximum_archive_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveReceipt {
    pub archive_path: PathBuf,
    pub outer_digest_path: PathBuf,
    pub archive_sha256: String,
    pub archive_bytes: u64,
    pub input_files: usize,
    pub input_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveVerification {
    pub archive_path: PathBuf,
    pub outer_digest_path: PathBuf,
    pub archive_sha256: String,
    pub archive_bytes: u64,
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("archive input or output path violates the immutable seal contract")]
    Path,
    #[error("archive input contains a symlink, hardlink, device, or ambiguous name")]
    FileType,
    #[error("archive input or output exceeds its frozen file or byte limit")]
    Limit,
    #[error("archive input changed while it was read")]
    Changed,
    #[error("archive I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
struct SourceFile {
    path: PathBuf,
    archive_path: String,
    size: u64,
}

struct LimitedWriter {
    file: File,
    written: u64,
    maximum: u64,
}

impl Write for LimitedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.written);
        if remaining == 0 {
            return Err(io::Error::other("frozen archive byte limit exceeded"));
        }
        let allowed = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let written = self.file.write(&buffer[..allowed])?;
        self.written = self.written.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub fn create_deterministic_archive(
    input_root: &Path,
    output_directory: &Path,
    limits: ArchiveLimits,
) -> Result<ArchiveReceipt, ArchiveError> {
    if limits.maximum_files == 0
        || limits.maximum_uncompressed_bytes == 0
        || limits.maximum_archive_bytes == 0
        || output_directory.exists()
    {
        return Err(ArchiveError::Path);
    }
    let input_root = fs::canonicalize(input_root)?;
    if !input_root.is_dir() {
        return Err(ArchiveError::Path);
    }
    let output_parent = fs::canonicalize(output_directory.parent().ok_or(ArchiveError::Path)?)?;
    let output_name = output_directory.file_name().ok_or(ArchiveError::Path)?;
    let resolved_output = output_parent.join(output_name);
    let output_name = output_name.to_str().ok_or(ArchiveError::Path)?;
    if output_name.is_empty()
        || output_name.contains(['/', '\\', ':', '\0', '\n', '\r'])
        || resolved_output.starts_with(&input_root)
    {
        return Err(ArchiveError::Path);
    }
    let staging_output = output_parent.join(format!(".{output_name}.building"));
    if staging_output.exists() {
        return Err(ArchiveError::Path);
    }

    let sources = collect_sources(&input_root, limits)?;
    let input_bytes = sources.iter().try_fold(0_u64, |total, source| {
        total.checked_add(source.size).ok_or(ArchiveError::Limit)
    })?;
    fs::create_dir(&staging_output)?;
    sync_directory(&output_parent)?;

    let archive_path = staging_output.join(ARCHIVE_NAME);
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&archive_path)?;
    let output = LimitedWriter {
        file: output,
        written: 0,
        maximum: limits.maximum_archive_bytes,
    };
    let mut encoder = zstd::Encoder::new(output, 19)?;
    encoder.include_checksum(true)?;
    let mut builder = Builder::new(encoder);
    for source in &sources {
        append_source(&mut builder, source)?;
    }
    let encoder = builder.into_inner()?;
    let output = encoder.finish()?;
    output.file.sync_all()?;

    let archive_bytes = output.written;
    drop(output);
    if archive_bytes == 0 || archive_bytes > limits.maximum_archive_bytes {
        return Err(ArchiveError::Limit);
    }
    let archive_sha256 = sha256_file(&archive_path)?;
    let outer_digest_path = staging_output.join(OUTER_DIGEST_NAME);
    let mut digest = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&outer_digest_path)?;
    digest.write_all(archive_sha256.as_bytes())?;
    digest.write_all(b"\n")?;
    digest.sync_all()?;
    drop(digest);
    sync_directory(&staging_output)?;
    make_tree_read_only(&staging_output)?;
    fs::rename(&staging_output, &resolved_output)?;
    sync_directory(&output_parent)?;
    verify_archive(
        &resolved_output,
        &archive_sha256,
        archive_bytes,
        limits.maximum_archive_bytes,
    )?;
    Ok(ArchiveReceipt {
        archive_path: resolved_output.join(ARCHIVE_NAME),
        outer_digest_path: resolved_output.join(OUTER_DIGEST_NAME),
        archive_sha256,
        archive_bytes,
        input_files: sources.len(),
        input_bytes,
    })
}

pub fn verify_archive(
    output_directory: &Path,
    expected_sha256: &str,
    expected_bytes: u64,
    maximum_archive_bytes: u64,
) -> Result<ArchiveVerification, ArchiveError> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || expected_bytes == 0
        || expected_bytes > maximum_archive_bytes
    {
        return Err(ArchiveError::Limit);
    }
    let metadata = fs::symlink_metadata(output_directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ArchiveError::Path);
    }
    let output_directory = fs::canonicalize(output_directory)?;
    if !verify_tree_read_only(&output_directory)? {
        return Err(ArchiveError::Changed);
    }
    let entries = fs::read_dir(&output_directory)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    if entries
        != std::collections::BTreeSet::from([
            std::ffi::OsString::from(ARCHIVE_NAME),
            std::ffi::OsString::from(OUTER_DIGEST_NAME),
        ])
    {
        return Err(ArchiveError::Path);
    }
    let archive_path = output_directory.join(ARCHIVE_NAME);
    let archive_metadata = fs::symlink_metadata(&archive_path)?;
    if !archive_metadata.is_file()
        || archive_metadata.file_type().is_symlink()
        || has_multiple_links(&archive_path, &archive_metadata)?
        || archive_metadata.len() != expected_bytes
    {
        return Err(ArchiveError::Changed);
    }
    let archive_sha256 = sha256_file(&archive_path)?;
    if archive_sha256 != expected_sha256 {
        return Err(ArchiveError::Changed);
    }
    let outer_digest_path = output_directory.join(OUTER_DIGEST_NAME);
    let outer_metadata = fs::symlink_metadata(&outer_digest_path)?;
    if !outer_metadata.is_file()
        || outer_metadata.file_type().is_symlink()
        || has_multiple_links(&outer_digest_path, &outer_metadata)?
        || outer_metadata.len() != 65
        || fs::read(&outer_digest_path)? != format!("{expected_sha256}\n").as_bytes()
    {
        return Err(ArchiveError::Changed);
    }
    Ok(ArchiveVerification {
        archive_path,
        outer_digest_path,
        archive_sha256,
        archive_bytes: expected_bytes,
    })
}

fn collect_sources(
    input_root: &Path,
    limits: ArchiveLimits,
) -> Result<Vec<SourceFile>, ArchiveError> {
    let mut sources = Vec::new();
    visit(input_root, input_root, &mut sources)?;
    sources.sort_by(|left, right| {
        left.archive_path
            .as_bytes()
            .cmp(right.archive_path.as_bytes())
    });
    let bytes = sources.iter().try_fold(0_u64, |total, source| {
        total.checked_add(source.size).ok_or(ArchiveError::Limit)
    })?;
    if sources.is_empty()
        || sources.len() > limits.maximum_files
        || bytes > limits.maximum_uncompressed_bytes
    {
        return Err(ArchiveError::Limit);
    }
    Ok(sources)
}

fn visit(root: &Path, directory: &Path, sources: &mut Vec<SourceFile>) -> Result<(), ArchiveError> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(ArchiveError::FileType);
        }
        if metadata.is_dir() {
            visit(root, &path, sources)?;
        } else if metadata.is_file() && !has_multiple_links(&path, &metadata)? {
            sources.push(SourceFile {
                archive_path: normalized_relative(root, &path)?,
                path,
                size: metadata.len(),
            });
        } else {
            return Err(ArchiveError::FileType);
        }
    }
    Ok(())
}

fn normalized_relative(root: &Path, path: &Path) -> Result<String, ArchiveError> {
    let relative = path.strip_prefix(root).map_err(|_| ArchiveError::Path)?;
    let mut components = Vec::new();
    for component in relative.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(ArchiveError::Path);
        };
        let component = component.to_str().ok_or(ArchiveError::Path)?;
        if component.is_empty()
            || component.contains('\0')
            || component.contains('/')
            || component.contains('\\')
        {
            return Err(ArchiveError::Path);
        }
        components.push(component);
    }
    let normalized = components.join("/");
    if normalized.is_empty() || normalized.len() > 1024 {
        return Err(ArchiveError::Path);
    }
    Ok(normalized)
}

fn append_source<W: Write>(
    builder: &mut Builder<W>,
    source: &SourceFile,
) -> Result<(), ArchiveError> {
    let mut file = File::open(&source.path)?;
    let before = file.metadata()?;
    if !before.is_file()
        || before.len() != source.size
        || has_multiple_links(&source.path, &before)?
    {
        return Err(ArchiveError::Changed);
    }
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Regular);
    header.set_size(source.size);
    header.set_mode(0o444);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder.append_data(&mut header, &source.archive_path, &mut file)?;
    let after = file.metadata()?;
    if after.len() != before.len() || has_multiple_links(&source.path, &after)? {
        return Err(ArchiveError::Changed);
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, ArchiveError> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex(&digest.finalize()))
}

#[cfg(unix)]
fn has_multiple_links(_path: &Path, metadata: &fs::Metadata) -> Result<bool, ArchiveError> {
    use std::os::unix::fs::MetadataExt;
    Ok(metadata.nlink() > 1)
}

#[cfg(windows)]
fn has_multiple_links(path: &Path, _metadata: &fs::Metadata) -> Result<bool, ArchiveError> {
    use std::mem::zeroed;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };

    let file = File::open(path)?;
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { zeroed() };
    // SAFETY: the file handle and writable information structure remain valid for the call.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(information.nNumberOfLinks > 1)
}

#[cfg(not(any(unix, windows)))]
fn has_multiple_links(_path: &Path, _metadata: &fs::Metadata) -> Result<bool, ArchiveError> {
    Ok(false)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), ArchiveError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), ArchiveError> {
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}
