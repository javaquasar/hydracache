use super::*;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

fn object(parent: &File, name: &str, directory: bool) -> Object {
    if directory {
        mkdir(parent, name).unwrap();
    }
    let file = open(
        parent,
        name,
        if directory {
            libc::O_RDONLY | libc::O_DIRECTORY
        } else {
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL
        },
    )
    .unwrap();
    let mut object = Object::retain(file, directory);
    object.initialize(unsafe { libc::geteuid() }, unsafe { libc::getegid() }).unwrap_or_else(|error| {
        panic!("kernel object initialization: {error:?} directory={directory} metadata={:?} status={:#x} fd={:#x}",
            object.file.metadata().unwrap(), descriptor_flags(&object.file, libc::F_GETFL).unwrap(), descriptor_flags(&object.file, libc::F_GETFD).unwrap());
    });
    object
}
#[test]
fn original_sink_kernel_exclusive_metadata_and_inventory() {
    let temp = tempfile::tempdir().unwrap();
    let parent = File::open(temp.path()).unwrap();
    let directory = object(&parent, DIRECTORY, true);
    inventory(&directory.file, &[]).unwrap();
    let sink = object(&directory.file, NAMES[0], false);
    sink.inspect(
        &directory.file,
        NAMES[0],
        unsafe { libc::geteuid() },
        unsafe { libc::getegid() },
    )
    .unwrap();
    inventory(&directory.file, &NAMES[..1]).unwrap();
    assert!(matches!(
        mkdir(&parent, DIRECTORY),
        Err(OriginalSinkError::Conflict)
    ));
    assert!(open(
        &directory.file,
        NAMES[0],
        libc::O_RDWR | libc::O_CREAT | libc::O_EXCL
    )
    .is_err());
    assert_eq!(sink.file.metadata().unwrap().len(), 0);
    fs::write(temp.path().join(DIRECTORY).join("unknown"), b"retained").unwrap();
    assert!(matches!(
        inventory(&directory.file, &NAMES[..1]),
        Err(OriginalSinkError::Invalid)
    ));
}
#[test]
fn original_sink_kernel_named_content_flags_and_links_refuse() {
    for kind in 0..8 {
        let temp = tempfile::tempdir().unwrap();
        let parent = File::open(temp.path()).unwrap();
        let sink = object(&parent, NAMES[0], false);
        let path = temp.path().join(NAMES[0]);
        match kind {
            0 => fs::write(&path, b"unowned bytes").unwrap(),
            1 => fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap(),
            2 => {
                fs::hard_link(&path, temp.path().join("alias")).unwrap();
            }
            3 => {
                fs::rename(&path, temp.path().join("old")).unwrap();
                fs::write(&path, []).unwrap();
            }
            4 => {
                fs::rename(&path, temp.path().join("old")).unwrap();
                symlink("old", &path).unwrap();
            }
            5 => set_flags(&sink.file, libc::F_SETFD, 0).unwrap(),
            6 => set_flags(&sink.file, libc::F_SETFL, libc::O_APPEND | libc::O_NONBLOCK).unwrap(),
            7 => set_flags(&sink.file, libc::F_SETFL, 0).unwrap(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                sink.inspect(&parent, NAMES[0], unsafe { libc::geteuid() }, unsafe {
                    libc::getegid()
                }),
                Err(OriginalSinkError::Invalid)
            ),
            "kind={kind}"
        );
    }
}
#[test]
fn original_sink_kernel_seeded_names_bind_exact_reserved_bytes() {
    let mut seed = 0x0740_5171_u64;
    for index in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let raw = [seed.to_le_bytes(), (index as u64).to_le_bytes()].concat();
        let a = output_name(&raw);
        assert_eq!(a.len(), PREFIX.len() + 64);
        assert!(a.starts_with(PREFIX));
        assert!(crate::is_hash(&a[PREFIX.len()..]));
        assert!(!a.contains(['/', '\\', '\0']));
        assert_eq!(a, output_name(&raw));
        assert_ne!(a, output_name(&[raw.as_slice(), b"\n"].concat()));
    }
    println!("HC074-SINK-NAMES seed=0x07405171 randomized=4096 exact=true");
}

#[test]
fn original_sink_kernel_concurrent_creation_and_directory_substitution() {
    let temp = tempfile::tempdir().unwrap();
    let parent = std::sync::Arc::new(File::open(temp.path()).unwrap());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads = (0..2)
        .map(|_| {
            let parent = parent.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                mkdir(&parent, DIRECTORY)
            })
        })
        .collect::<Vec<_>>();
    let results = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(OriginalSinkError::Conflict)))
            .count(),
        1
    );
    let mut directory = Object::retain(
        open(&parent, DIRECTORY, libc::O_RDONLY | libc::O_DIRECTORY).unwrap(),
        true,
    );
    directory
        .initialize(unsafe { libc::geteuid() }, unsafe { libc::getegid() })
        .unwrap();
    inventory(&directory.file, &[]).unwrap();
    fs::rename(temp.path().join(DIRECTORY), temp.path().join("original")).unwrap();
    mkdir(&parent, DIRECTORY).unwrap();
    assert!(matches!(
        directory.inspect(&parent, DIRECTORY, unsafe { libc::geteuid() }, unsafe {
            libc::getegid()
        }),
        Err(OriginalSinkError::Invalid)
    ));
    inventory(&directory.file, &[]).unwrap();
}
