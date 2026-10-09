// Read-only local audit. Synthetic identity is not a lease or host admission.
use hydracache_long_run_supervisor_074::{
    diagnostic_artifacts::{ArtifactContents, ArtifactDigest},
    diagnostic_builder::{decode_key, load_policy},
    diagnostic_lease::DiagnosticIdentity,
};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read},
    path::Path,
};

const PIN: &str = "dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22";
const CONTROLLER: &str = "c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda";

fn read(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bounded regular file required",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bounded read refused",
        ));
    }
    Ok(bytes)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("expected independently retained policy, receipt, unsigned bundle".into());
    }
    let policy_bytes = read(Path::new(&args[0]), 4096)?;
    let controller = decode_key(CONTROLLER)?;
    let policy = load_policy(&policy_bytes, PIN, &controller)?;
    assert!(load_policy(&policy_bytes, &"0".repeat(64), &controller).is_err());
    let receipt = read(Path::new(&args[1]), 65_536)?;
    let bundle = Path::new(&args[2]);
    let binary = read(&bundle.join("timing-controls-074"), 134_217_728)?;
    let root_lock = read(&bundle.join("Cargo.lock.root"), 1_048_576)?;
    let observer_lock = read(&bundle.join("Cargo.lock.observer"), 1_048_576)?;
    let build_log = read(&bundle.join("build-log.jsonl"), 16_777_216)?;
    let mut owned_configs = BTreeMap::new();
    for surface in ["embedded", "direct", "resp2", "resp3"] {
        owned_configs.insert(
            surface.to_owned(),
            read(&bundle.join(format!("{surface}.json")), 65_536)?,
        );
    }
    let mut identity = DiagnosticIdentity {
        lease_id: "1".repeat(64),
        boot_id: "00000000-0000-0000-0000-000000000000".into(),
        binary_sha256: ArtifactDigest::of(&binary).sha256,
        build_provenance_sha256: ArtifactDigest::of(&receipt).sha256,
    };
    let verified = policy.verify_receipt(&receipt, &identity)?;
    let mut contents = ArtifactContents {
        binary: &binary,
        root_lock: &root_lock,
        observer_lock: &observer_lock,
        build_log: &build_log,
        configs: owned_configs
            .iter()
            .map(|(k, v)| (k.clone(), v.as_slice()))
            .collect(),
    };
    verified.verify_contents(&contents)?;
    let mut changed_binary = binary.clone();
    let last = changed_binary.last_mut().unwrap();
    *last ^= 1;
    contents.binary = &changed_binary;
    assert!(verified.verify_contents(&contents).is_err());
    let marker = b"\"signature_hex\":\"";
    let offset = receipt
        .windows(marker.len())
        .position(|part| part == marker)
        .ok_or("signature field absent")?
        + marker.len();
    let mut changed_receipt = receipt.clone();
    changed_receipt[offset] = if changed_receipt[offset] == b'0' {
        b'1'
    } else {
        b'0'
    };
    identity.build_provenance_sha256 = ArtifactDigest::of(&changed_receipt).sha256;
    assert!(policy.verify_receipt(&changed_receipt, &identity).is_err());
    println!("signature_verified=true contents_verified=true negative_guards_passed=true execution_authorized=false admission_allowed=false");
    println!(
        "source_commit={} source_tree={}",
        verified.statement().source_commit,
        verified.statement().source_tree
    );
    println!(
        "binary_sha256={} receipt_sha256={}",
        ArtifactDigest::of(&binary).sha256,
        ArtifactDigest::of(&receipt).sha256
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_reader_accepts_exact_limit_and_refuses_unsafe_leaves() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "hydracache-receipt-reader-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let file = root.join("file");
        fs::write(&file, b"abcd").unwrap();
        assert_eq!(read(&file, 4).unwrap(), b"abcd");
        assert!(read(&file, 3).is_err());
        assert!(read(&root, 4).is_err());
        assert!(read(&root.join("absent"), 4).is_err());
        fs::write(&file, b"").unwrap();
        assert!(read(&file, 4).is_err());
        #[cfg(unix)]
        {
            let alias = root.join("alias");
            std::os::unix::fs::symlink(&file, &alias).unwrap();
            assert!(read(&alias, 4).is_err());
        }
        fs::remove_dir_all(&root).unwrap();
    }
}
