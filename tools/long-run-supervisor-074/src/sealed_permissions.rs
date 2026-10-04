#[cfg(unix)]
use std::fs;
use std::io;
use std::path::Path;

#[cfg(unix)]
pub(crate) fn make_tree_read_only(root: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::other("sealed root is not a safe directory"));
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other("sealed tree contains a symlink"));
        }
        if metadata.is_dir() {
            make_tree_read_only(&path)?;
        } else if metadata.is_file() {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o400))?;
            fs::File::open(&path)?.sync_all()?;
        } else {
            return Err(io::Error::other("sealed tree contains a special file"));
        }
    }
    fs::set_permissions(root, fs::Permissions::from_mode(0o500))?;
    fs::File::open(root)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn make_tree_read_only(_root: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
pub(crate) fn verify_tree_read_only(root: &Path) -> io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o777 != 0o500
    {
        return Ok(false);
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Ok(false);
        }
        if metadata.is_dir() {
            if !verify_tree_read_only(&path)? {
                return Ok(false);
            }
        } else if !metadata.is_file() || metadata.permissions().mode() & 0o777 != 0o400 {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(not(unix))]
pub(crate) fn verify_tree_read_only(_root: &Path) -> io::Result<bool> {
    Ok(true)
}
