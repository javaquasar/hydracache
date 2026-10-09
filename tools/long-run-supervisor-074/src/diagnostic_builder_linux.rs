//! Protected signing-file boundary; no builder subprocess, key generator or installer.
use super::*;
use crate::diagnostic_artifacts::{
    receipt_bytes, MAX_BINARY_BYTES, MAX_CONFIG_BYTES, MAX_LOCK_BYTES, MAX_LOG_BYTES,
    MAX_RECEIPT_BYTES,
};
use crate::diagnostic_lease::SURFACES;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

fn invalid<T>() -> Result<T, ArtifactError> {
    Err(ArtifactError::Invalid)
}

// Walk absolute parents through O_NOFOLLOW descriptors. A trusted root-owned
// sticky temporary ancestor is allowed; the immediate secret/output directory
// itself must be owned by the signer and private. Root/current UID are trusted.
fn parent(path: &Path, private: bool) -> Result<(File, CString), ArtifactError> {
    let Some(directory) = path.parent() else {
        return invalid();
    };
    if !path.is_absolute() {
        return invalid();
    }
    let mut fd = File::open("/")?;
    // SAFETY: getter without pointer arguments.
    let uid = unsafe { libc::geteuid() };
    for component in directory.components() {
        let name = match component {
            Component::RootDir => continue,
            Component::Normal(name) => {
                CString::new(name.as_bytes()).map_err(|_| ArtifactError::Invalid)?
            }
            _ => return invalid(),
        };
        // SAFETY: owned directory FD and valid NUL-terminated component.
        let next = unsafe {
            libc::openat(
                fd.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if next < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: successful openat returned a new owned descriptor.
        fd = unsafe { File::from_raw_fd(next) };
        let m = fd.metadata()?;
        if (m.uid() != 0 && m.uid() != uid)
            || (m.mode() & 0o022 != 0 && !(m.uid() == 0 && m.mode() & 0o1000 != 0))
        {
            return invalid();
        }
    }
    let m = fd.metadata()?;
    if private && (m.uid() != uid || m.mode() & 0o7777 != 0o700) {
        return invalid();
    }
    let leaf = path.file_name().ok_or(ArtifactError::Invalid)?;
    Ok((
        fd,
        CString::new(leaf.as_bytes()).map_err(|_| ArtifactError::Invalid)?,
    ))
}

fn stamp(m: &std::fs::Metadata) -> [u64; 11] {
    [
        m.dev(),
        m.ino(),
        m.uid() as u64,
        m.gid() as u64,
        m.mode() as u64,
        m.nlink(),
        m.len(),
        m.mtime() as u64,
        m.mtime_nsec() as u64,
        m.ctime() as u64,
        m.ctime_nsec() as u64,
    ]
}

fn read(path: &Path, limit: u64, secret: bool) -> Result<Vec<u8>, ArtifactError> {
    let (parent, name) = parent(path, secret)?;
    // SAFETY: valid retained directory FD and NUL-terminated leaf.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: openat returned a new owned descriptor.
    let mut file = unsafe { File::from_raw_fd(fd) };
    let before = file.metadata()?;
    // SAFETY: getter without pointer arguments.
    let uid = unsafe { libc::geteuid() };
    if !before.is_file()
        || before.nlink() != 1
        || before.len() == 0
        || before.len() > limit
        || (secret && (before.uid() != uid || before.mode() & 0o7777 != 0o600))
    {
        return invalid();
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != before.len() || stamp(&before) != stamp(&file.metadata()?) {
        return invalid();
    }
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
    let (directory, name) = parent(path, true)?;
    // SAFETY: retained directory FD and NUL-terminated leaf; never overwrite.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returned a new owned descriptor.
    let mut output = unsafe { File::from_raw_fd(fd) };
    output.write_all(bytes)?;
    output.sync_all()?;
    directory.sync_all()?;
    Ok(())
}

/// Input bundle is supplied only by the reviewed build job, not a host/client.
pub fn sign_files(
    policy: &Path,
    pin: &str,
    controller_hex: &str,
    key: &Path,
    observation: &Path,
    bundle: &Path,
    output: &Path,
) -> Result<(), ArtifactError> {
    let policy = load_policy(
        &read(policy, MAX_POLICY_BYTES as u64, false)?,
        pin,
        &decode_key(controller_hex)?,
    )?;
    let observed: BuildObservation =
        serde_json::from_slice(&read(observation, MAX_RECEIPT_BYTES, false)?)?;
    let binary = read(&bundle.join("timing-controls-074"), MAX_BINARY_BYTES, false)?;
    let root_lock = read(&bundle.join("Cargo.lock.root"), MAX_LOCK_BYTES, false)?;
    let observer_lock = read(&bundle.join("Cargo.lock.observer"), MAX_LOCK_BYTES, false)?;
    let log = read(&bundle.join("build-log.jsonl"), MAX_LOG_BYTES, false)?;
    let configs: BTreeMap<String, Vec<u8>> = SURFACES
        .iter()
        .map(|name| {
            Ok((
                name.to_string(),
                read(
                    &bundle.join(format!("{name}.json")),
                    MAX_CONFIG_BYTES,
                    false,
                )?,
            ))
        })
        .collect::<Result<_, ArtifactError>>()?;
    let mut secret = read(key, 65, true)?;
    let raw = secret.strip_suffix(b"\n").unwrap_or(&secret);
    // This parser requires lowercase hexadecimal, like the public policy.
    let decoded = std::str::from_utf8(raw)
        .ok()
        .filter(|v| crate::is_hash(v))
        .map(|v| {
            let mut bytes = [0u8; 32];
            for (i, pair) in v.as_bytes().chunks_exact(2).enumerate() {
                let n = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
                bytes[i] = (n(pair[0]) << 4) | n(pair[1]);
            }
            bytes
        });
    // Clear the encoded temporary; SigningKey owns/zeroizes its secret material.
    secret.fill(0);
    let mut decoded = decoded.ok_or(ArtifactError::Invalid)?;
    let signing_key = SigningKey::from_bytes(&decoded);
    decoded.fill(0);
    let receipt = sign_build(
        &policy,
        &signing_key,
        &observed,
        &ArtifactContents {
            binary: &binary,
            root_lock: &root_lock,
            observer_lock: &observer_lock,
            build_log: &log,
            configs: configs
                .iter()
                .map(|(k, v)| (k.clone(), v.as_slice()))
                .collect(),
        },
    )?;
    write_new(output, &receipt_bytes(&receipt)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    #[test]
    fn builder_cli_refuses_unprotected_key_files_and_existing_outputs() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = temp.path().join("key");
        std::fs::write(&path, b"a").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(read(&path, 65, true).unwrap(), b"a");
        assert!(read(Path::new("relative-key"), 65, true).is_err());
        assert!(read(temp.path(), 65, false).is_err());
        let directory_alias = temp.path().join("directory-alias");
        symlink(temp.path(), &directory_alias).unwrap();
        assert!(read(&directory_alias.join("key"), 65, true).is_err());
        let alias = temp.path().join("alias");
        symlink(&path, &alias).unwrap();
        assert!(read(&alias, 65, true).is_err());
        std::fs::hard_link(&path, temp.path().join("hardlink")).unwrap();
        assert!(read(&path, 65, true).is_err());
        let other = temp.path().join("other");
        std::fs::write(&other, [1u8; 66]).unwrap();
        std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(read(&other, 65, true).is_err());
        std::fs::write(&other, b"").unwrap();
        assert!(read(&other, 65, false).is_err());
        std::fs::write(&other, b"a").unwrap();
        std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read(&other, 65, true).is_err());
        let output = temp.path().join("receipt");
        write_new(&output, b"original").unwrap();
        assert!(write_new(&output, b"replacement").is_err());
        assert_eq!(std::fs::read(&output).unwrap(), b"original");
        assert_eq!(std::fs::metadata(&output).unwrap().mode() & 0o777, 0o600);
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(write_new(&temp.path().join("new"), b"x").is_err());
    }
}
