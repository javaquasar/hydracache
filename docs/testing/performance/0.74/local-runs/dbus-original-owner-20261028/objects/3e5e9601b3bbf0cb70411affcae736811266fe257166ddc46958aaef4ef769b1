use hydracache_long_run_supervisor_074::archive::{
    create_deterministic_archive, verify_archive, ArchiveError, ArchiveLimits, ARCHIVE_NAME,
    OUTER_DIGEST_NAME,
};
use sha2::{Digest, Sha256};
use std::fs::{self, File};

fn limits() -> ArchiveLimits {
    ArchiveLimits {
        maximum_files: 20_000,
        maximum_uncompressed_bytes: 21_474_836_480,
        maximum_archive_bytes: 21_474_836_480,
    }
}

fn fixture(root: &std::path::Path, reverse: bool) {
    fs::create_dir_all(root.join("raw/nested")).unwrap();
    let files = [
        ("packet-manifest.json", b"{\"schema_version\":1}".as_slice()),
        ("raw/a.txt", b"alpha".as_slice()),
        ("raw/nested/b.bin", b"beta\0bytes".as_slice()),
    ];
    if reverse {
        for (name, bytes) in files.iter().rev() {
            fs::write(root.join(name), bytes).unwrap();
        }
    } else {
        for (name, bytes) in files {
            fs::write(root.join(name), bytes).unwrap();
        }
    }
}

#[test]
fn archive_bytes_and_headers_are_deterministic() {
    let temporary = tempfile::tempdir().unwrap();
    let first = temporary.path().join("first");
    let second = temporary.path().join("second");
    fixture(&first, false);
    fixture(&second, true);
    let first_output = temporary.path().join("first-seal");
    let second_output = temporary.path().join("second-seal");
    let first_receipt = create_deterministic_archive(&first, &first_output, limits()).unwrap();
    let second_receipt = create_deterministic_archive(&second, &second_output, limits()).unwrap();

    let first_bytes = fs::read(&first_receipt.archive_path).unwrap();
    let second_bytes = fs::read(&second_receipt.archive_path).unwrap();
    assert_eq!(first_bytes, second_bytes);
    let digest = Sha256::digest(&first_bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(first_receipt.archive_sha256, digest);
    assert_eq!(
        fs::read_to_string(&first_receipt.outer_digest_path).unwrap(),
        format!("{digest}\n")
    );
    assert_eq!(first_receipt.input_files, 3);
    assert_eq!(first_receipt.input_bytes, 35);
    let verified = verify_archive(
        &first_output,
        &first_receipt.archive_sha256,
        first_receipt.archive_bytes,
        limits().maximum_archive_bytes,
    )
    .unwrap();
    assert_eq!(verified.archive_sha256, first_receipt.archive_sha256);

    let decoder = zstd::Decoder::new(File::open(first_receipt.archive_path).unwrap()).unwrap();
    let mut archive = tar::Archive::new(decoder);
    let mut paths = Vec::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        assert_eq!(entry.header().mtime().unwrap(), 0);
        assert_eq!(entry.header().uid().unwrap(), 0);
        assert_eq!(entry.header().gid().unwrap(), 0);
        assert_eq!(entry.header().mode().unwrap(), 0o444);
        paths.push(entry.path().unwrap().to_string_lossy().into_owned());
    }
    assert_eq!(
        paths,
        ["packet-manifest.json", "raw/a.txt", "raw/nested/b.bin"]
    );
    assert!(first_output.join(ARCHIVE_NAME).is_file());
    assert!(first_output.join(OUTER_DIGEST_NAME).is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&first_output).unwrap().permissions().mode() & 0o777,
            0o500
        );
        assert_eq!(
            fs::metadata(first_output.join(ARCHIVE_NAME))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o400
        );
        assert_eq!(
            fs::metadata(first_output.join(OUTER_DIGEST_NAME))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o400
        );
    }
}

#[test]
fn archive_replay_verification_rejects_digest_and_extra_file_drift() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    fixture(&input, false);
    let output = temporary.path().join("seal");
    let receipt = create_deterministic_archive(&input, &output, limits()).unwrap();
    assert!(verify_archive(
        &output,
        &"f".repeat(64),
        receipt.archive_bytes,
        limits().maximum_archive_bytes,
    )
    .is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(output.join("unexpected"), b"drift").unwrap();
    assert!(verify_archive(
        &output,
        &receipt.archive_sha256,
        receipt.archive_bytes,
        limits().maximum_archive_bytes,
    )
    .is_err());
}

#[test]
fn create_new_limits_and_output_placement_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    fixture(&input, false);

    let too_small = ArchiveLimits {
        maximum_files: 2,
        ..limits()
    };
    let limited_output = temporary.path().join("limited");
    assert!(matches!(
        create_deterministic_archive(&input, &limited_output, too_small),
        Err(ArchiveError::Limit)
    ));
    assert!(!limited_output.exists());

    let compressed_limit = ArchiveLimits {
        maximum_archive_bytes: 16,
        ..limits()
    };
    let bounded_output = temporary.path().join("bounded");
    assert!(create_deterministic_archive(&input, &bounded_output, compressed_limit).is_err());
    assert!(!bounded_output.exists());
    let bounded_staging = temporary.path().join(".bounded.building");
    assert!(
        fs::metadata(bounded_staging.join(ARCHIVE_NAME))
            .unwrap()
            .len()
            <= 16
    );
    assert!(!bounded_staging.join(OUTER_DIGEST_NAME).exists());

    let nested_output = input.join("seal");
    assert!(matches!(
        create_deterministic_archive(&input, &nested_output, limits()),
        Err(ArchiveError::Path)
    ));

    let existing = temporary.path().join("existing");
    fs::create_dir(&existing).unwrap();
    assert!(matches!(
        create_deterministic_archive(&input, &existing, limits()),
        Err(ArchiveError::Path)
    ));
}

#[test]
fn hardlinked_input_is_rejected_without_creating_an_archive() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("input");
    fixture(&input, false);
    fs::hard_link(input.join("raw/a.txt"), input.join("raw/alias.txt")).unwrap();
    let output = temporary.path().join("seal");
    assert!(matches!(
        create_deterministic_archive(&input, &output, limits()),
        Err(ArchiveError::FileType)
    ));
    assert!(!output.exists());
}
