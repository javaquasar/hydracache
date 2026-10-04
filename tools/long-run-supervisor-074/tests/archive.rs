use hydracache_long_run_supervisor_074::archive::{
    create_deterministic_archive, ArchiveError, ArchiveLimits, ARCHIVE_NAME, OUTER_DIGEST_NAME,
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
    assert!(
        fs::metadata(bounded_output.join(ARCHIVE_NAME))
            .unwrap()
            .len()
            <= 16
    );
    assert!(!bounded_output.join(OUTER_DIGEST_NAME).exists());

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
