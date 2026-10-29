//! Explicit local byte/declaration inventory, never original launch or custody.
#[cfg(target_os = "linux")]
fn read_input(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
    const MAX: u64 = 8 * 1024 * 1024;
    let refused =
        || std::io::Error::new(std::io::ErrorKind::InvalidData, "input inventory refused");
    if !path.is_absolute() {
        return Err(refused());
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?;
    let stamp = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.uid(),
            m.gid(),
            m.mode(),
            m.nlink(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    let before = file.metadata()?;
    if !before.is_file() || before.nlink() != 1 || !(64..=MAX).contains(&before.len()) {
        return Err(refused());
    }
    let mut bytes = vec![0u8; before.len() as usize + 1];
    let mut count = 0;
    while count < bytes.len() {
        let n = file.read_at(&mut bytes[count..], count as u64)?;
        if n == 0 {
            break;
        }
        count += n;
    }
    if count as u64 != before.len() || stamp(&before) != stamp(&file.metadata()?) {
        return Err(refused());
    }
    bytes.truncate(count);
    Ok(bytes)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use hydracache_long_run_supervisor_074::diagnostic_signer_gnu::inspect_gnu_declarations;
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() != 4 {
            return Err("expected explicit signer/loader/libc/libgcc paths".into());
        }
        let inputs = args
            .iter()
            .map(|p| read_input(std::path::Path::new(p)))
            .collect::<std::io::Result<Vec<_>>>()?;
        let catalog = inspect_gnu_declarations([&inputs[0], &inputs[1], &inputs[2], &inputs[3]])?;
        println!("{}", serde_json::to_string(&catalog)?);
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("file inventory requires Linux".into())
    }
}
fn main() {
    if run().is_err() {
        eprintln!("GNU signer catalog refused; no custody or execution granted.");
        std::process::exit(2);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    #[test]
    fn bounded_regular_original_reads_do_not_change_bytes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("public-input");
        std::fs::write(&path, vec![7u8; 128]).unwrap();
        assert_eq!(read_input(&path).unwrap(), vec![7u8; 128]);
        assert_eq!(std::fs::read(&path).unwrap(), vec![7u8; 128]);
    }
    #[test]
    fn relative_symlink_special_hardlink_and_size_refuse() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("public-input");
        assert!(read_input(std::path::Path::new("relative")).is_err());
        assert!(read_input(root.path()).is_err());
        for size in [0, 63, 8 * 1024 * 1024 + 1] {
            std::fs::write(&path, vec![0u8; size]).unwrap();
            assert!(read_input(&path).is_err());
        }
        std::fs::write(&path, vec![0u8; 64]).unwrap();
        let link = root.path().join("alias");
        symlink(&path, &link).unwrap();
        assert!(read_input(&link).is_err());
        std::fs::hard_link(&path, root.path().join("hard")).unwrap();
        assert!(read_input(&path).is_err());
    }
}
