use hydracache_long_run_supervisor_074::archive::{
    create_deterministic_archive, verify_archive, ArchiveLimits,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn actual_archive_wire_is_deterministic_and_cross_decodes() {
    let mut observations = Vec::new();
    for (index, size) in [0, 1, 4095, 16384, 131072, 262144].into_iter().enumerate() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("input-a");
        let second = temp.path().join("input-b");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        let mut state = 7606312_u64;
        let payload: Vec<u8> = (0..size)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 32) as u8
            })
            .collect();
        for root in [&first, &second] {
            fs::write(root.join("data.bin"), &payload).unwrap();
            fs::write(root.join("manifest.json"), b"{\"fixed\":true}").unwrap();
        }
        let limits = ArchiveLimits {
            maximum_files: 2,
            maximum_uncompressed_bytes: 1_048_576,
            maximum_archive_bytes: 1_048_576,
        };
        let a = create_deterministic_archive(&first, &temp.path().join("a"), limits).unwrap();
        let b = create_deterministic_archive(&second, &temp.path().join("b"), limits).unwrap();
        let bytes = fs::read(&a.archive_path).unwrap();
        assert_eq!(bytes, fs::read(&b.archive_path).unwrap());
        verify_archive(
            a.archive_path.parent().unwrap(),
            &a.archive_sha256,
            a.archive_bytes,
            limits.maximum_archive_bytes,
        )
        .unwrap();
        let mut old = Vec::new();
        let mut new = Vec::new();
        zstd_old::Decoder::new(Cursor::new(&bytes))
            .unwrap()
            .take(2_097_152)
            .read_to_end(&mut old)
            .unwrap();
        zstd_new::Decoder::new(Cursor::new(&bytes))
            .unwrap()
            .take(2_097_152)
            .read_to_end(&mut new)
            .unwrap();
        assert_eq!(old, new);
        observations.push(serde_json::json!({"index":index,"size":size,"payload_sha256":hex(&Sha256::digest(&payload)),"archive_sha256":a.archive_sha256,"archive_bytes":a.archive_bytes,"tar_sha256":hex(&Sha256::digest(&old)),"tar_bytes":old.len()}));
    }
    println!(
        "HC074_SUPERVISOR_ZSTD:{}",
        serde_json::json!({"seed":7606312,"observations":observations,"performance_claim":false})
    );
}
