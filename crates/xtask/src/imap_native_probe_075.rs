//! Strict independent native observations. Does not admit production comparison capabilities.
use serde::{Deserialize, Serialize};
use std::{error::Error, path::Path, process::Command, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeProbe {
    pub schema: String,
    pub profile: String,
    pub observed_fields: Vec<String>,
    pub artifact_sha256: String,
    pub point_calls: u32,
    pub second_put_kinds: Vec<String>,
    pub second_put_previous: Vec<Option<String>>,
    pub size: u32,
    pub is_empty: bool,
    pub contains_entry: bool,
    pub fallback: String,
    pub scan_count: u32,
    pub native_version: u64,
    pub native_ttl_millis: u64,
    pub native_event_count: u32,
    pub snapshot_digest: String,
    pub removed_previous: Vec<Option<String>>,
    pub put_all_origin: String,
    pub clear_event: String,
    pub clear_affected: u32,
    pub final_digest: String,
}
pub fn validate(probe: &NativeProbe) -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    for (key, value) in [(vec![0, 255], vec![97]), (vec![97], vec![0, 128, 255])] {
        for value in [key, value] {
            bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
            bytes.extend(value);
        }
        bytes.push(0);
    }
    if probe.schema != "imap075-native-observation-v1"
        || probe.profile != "native-exclusive-eternal-v1"
        || probe.observed_fields != ["raw_return", "final_digest", "native_events"]
        || probe.artifact_sha256 != crate::imap_pre074_075::HAZELCAST_SHA256
        || probe.point_calls != 3
        || probe.second_put_kinds != ["replaced", "inserted"]
        || probe.second_put_previous != [Some("0080ff".into()), None]
        || probe.size != 2
        || probe.is_empty
        || !probe.contains_entry
        || probe.fallback != "0080ff"
        || probe.scan_count != 2
        || probe.native_event_count != 3
        || ![0, i64::MAX as u64].contains(&probe.native_ttl_millis)
        || probe.snapshot_digest != crate::imap_qualification_075::digest(&bytes)
        || probe.removed_previous != [Some("61".into()), None]
        || probe.put_all_origin != "native_aggregate"
        || probe.clear_event != "map_cleared"
        || probe.clear_affected != 1
        || probe.final_digest != crate::imap_qualification_075::digest(&[])
    {
        return Err("native observations differ from independent bounded specification".into());
    }
    Ok(())
}

/// Only code-selected packaged probes can be invoked; receipts cannot choose this classpath.
pub fn observe(root: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let bytes = execute_native(
        root,
        "io.hydracache.imap.hazelcast.NativeExtendedProbeMain075",
    )?;
    let parsed: NativeProbe = serde_json::from_slice(&bytes)?;
    validate(&parsed)?;
    Ok(bytes)
}
pub(crate) fn execute_native(root: &Path, class: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let paths = [
        root.join("tests/java-imap-benchmark/target/java-imap-benchmark-0.75.0-SNAPSHOT.jar"),
        root.join(
            "tests/java-imap-hazelcast-075/target/java-imap-hazelcast-075-0.75.0-SNAPSHOT.jar",
        ),
        root.join("tests/java-imap-hazelcast-075/target/dependencies/hazelcast-5.5.0.jar"),
    ];
    let hashes = paths
        .iter()
        .map(|p| crate::imap_daemon_probe_075::binary_digest(p))
        .collect::<Result<Vec<_>, _>>()?;
    if hashes[2] != crate::imap_pre074_075::HAZELCAST_SHA256 {
        return Err("native dependency pin differs".into());
    }
    let mut command = Command::new(crate::imap_api_differential::java_executable());
    command
        .arg("-Xmx128m")
        .arg(format!(
            "-Djdk.net.unixdomain.tmpdir={}",
            root.join("tests/java-imap-hazelcast-075/target").display()
        ))
        .arg("-cp")
        .arg(std::env::join_paths(&paths)?)
        .arg(class);
    let result = crate::imap_proof_process_075::execute(
        command,
        &[],
        crate::imap_proof_process_075::ProcessBounds {
            timeout: Duration::from_secs(40),
            stdout_bytes: 65536,
            ..Default::default()
        },
    )?;
    for (path, before) in paths.iter().zip(hashes) {
        if crate::imap_daemon_probe_075::binary_digest(path)? != before {
            return Err("native artifact changed while running".into());
        }
    }
    Ok(result.stdout)
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args != ["--release", "0.75"] {
        return Err("requires exactly --release 0.75".into());
    }
    let root = crate::doc_check::find_repo_root()?;
    println!("{}", String::from_utf8(observe(&root)?)?);
    Ok(())
}
