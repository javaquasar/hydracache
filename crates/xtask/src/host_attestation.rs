use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use hydracache_loadgen::profile::{
    MeasurementCore, RunnerAttestationV2, REFERENCE_FINGERPRINT_SCHEMA_VERSION,
    REFERENCE_HOST_CONTRACT_VERSION, REFERENCE_MEASUREMENT_CPUS, REFERENCE_OS_IMAGE,
    REFERENCE_STORAGE_CLASS,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAttestationInput {
    pub virtualization: String,
    pub physical_cores: u32,
    pub measurement_cores: Vec<MeasurementCore>,
    pub raw_host_identity: Vec<String>,
    pub storage_class: String,
    pub raw_storage_identity: Vec<String>,
    pub os_image: String,
    pub toolchain_identity: String,
    pub prebuild_contract_digest: String,
}

pub fn build_attestation(input: HostAttestationInput) -> Result<RunnerAttestationV2, String> {
    let host_digest = privacy_digest("hydracache-host-identity-v2", &input.raw_host_identity)?;
    let storage_identity_digest = privacy_digest(
        "hydracache-storage-identity-v2",
        &input.raw_storage_identity,
    )?;
    let attestation = RunnerAttestationV2 {
        schema_version: REFERENCE_FINGERPRINT_SCHEMA_VERSION,
        contract_version: REFERENCE_HOST_CONTRACT_VERSION.to_owned(),
        virtualization: input.virtualization,
        physical_cores: input.physical_cores,
        measurement_cores: input.measurement_cores,
        host_digest,
        storage_class: input.storage_class,
        storage_identity_digest,
        os_image: input.os_image,
        toolchain_identity: input.toolchain_identity,
        prebuild_contract_digest: input.prebuild_contract_digest,
    };
    let problems = hydracache_loadgen::profile::reference_attestation_problems(&attestation);
    if problems.is_empty() {
        Ok(attestation)
    } else {
        Err(format!("reference host attestation failed: {problems:?}"))
    }
}

pub fn observe_reference_attestation(
    toolchain_identity: &str,
    prebuild_contract_digest: &str,
) -> Result<RunnerAttestationV2, String> {
    let virtualization = detect_virtualization()?;
    let (physical_cores, measurement_cores) = observe_cpu_topology()?;
    let raw_host_identity = read_identity_values(&[
        "/sys/class/dmi/id/product_uuid",
        "/sys/class/dmi/id/board_serial",
        "/sys/class/dmi/id/product_serial",
    ]);
    let (storage_class, raw_storage_identity) = observe_root_storage()?;
    let os_image = observe_os_image()?;

    build_attestation(HostAttestationInput {
        virtualization,
        physical_cores,
        measurement_cores,
        raw_host_identity,
        storage_class,
        raw_storage_identity,
        os_image,
        toolchain_identity: toolchain_identity.to_owned(),
        prebuild_contract_digest: prebuild_contract_digest.to_owned(),
    })
}

fn detect_virtualization() -> Result<String, String> {
    let quiet = Command::new("systemd-detect-virt")
        .arg("--quiet")
        .output()
        .map_err(|error| format!("systemd-detect-virt probe failed: {error}"))?;
    match quiet.status.code() {
        Some(1) if quiet.stdout.is_empty() && quiet.stderr.is_empty() => Ok("none".to_owned()),
        Some(0) => {
            let named = command_output("systemd-detect-virt", &[])?;
            let technology = stdout_trimmed(&named, "systemd-detect-virt")?;
            Err(format!(
                "virtualization detected by systemd-detect-virt: {technology}"
            ))
        }
        code => Err(format!(
            "systemd-detect-virt returned unexpected status {code:?}: {}",
            String::from_utf8_lossy(&quiet.stderr).trim()
        )),
    }
}

fn observe_cpu_topology() -> Result<(u32, Vec<MeasurementCore>), String> {
    let cpu_root = Path::new("/sys/devices/system/cpu");
    let mut physical = BTreeSet::new();
    for entry in fs::read_dir(cpu_root).map_err(|error| format!("reading CPU topology: {error}"))? {
        let entry = entry.map_err(|error| format!("reading CPU topology entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(suffix) = name.strip_prefix("cpu") else {
            continue;
        };
        if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let topology = entry.path().join("topology");
        if !topology.is_dir() {
            continue;
        }
        physical.insert((
            read_u32(&topology.join("physical_package_id"))?,
            read_u32(&topology.join("core_id"))?,
        ));
    }
    let physical_cores =
        u32::try_from(physical.len()).map_err(|_| "physical core count overflow".to_owned())?;
    let mut measurement_cores = Vec::new();
    for logical_cpu in REFERENCE_MEASUREMENT_CPUS {
        let topology = cpu_root.join(format!("cpu{logical_cpu}/topology"));
        measurement_cores.push(MeasurementCore {
            logical_cpu,
            package_id: read_u32(&topology.join("physical_package_id"))?,
            core_id: read_u32(&topology.join("core_id"))?,
        });
    }
    Ok((physical_cores, measurement_cores))
}

fn observe_root_storage() -> Result<(String, Vec<String>), String> {
    let findmnt = command_output("findmnt", &["--noheadings", "--output", "SOURCE", "/"])?;
    let source = stdout_trimmed(&findmnt, "findmnt")?;
    if !source.starts_with("/dev/") {
        return Err(format!(
            "root filesystem source is not a local block device: {source:?}"
        ));
    }
    let lsblk = command_output(
        "lsblk",
        &[
            "--inverse",
            "--noheadings",
            "--paths",
            "--output",
            "NAME,TYPE,TRAN,MODEL,SERIAL,WWN",
            source,
        ],
    )?;
    let text = stdout_trimmed(&lsblk, "lsblk")?;
    let mut raw_identity = Vec::new();
    let mut disk_count = 0_u32;
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 3 || fields[1] != "disk" {
            continue;
        }
        disk_count += 1;
        if fields[2] != "nvme" {
            return Err(format!(
                "root storage leaf is not NVMe: {}",
                fields[..3].join(" ")
            ));
        }
        raw_identity.push(line.trim().to_owned());
    }
    if disk_count == 0 {
        return Err("root storage has no observable physical disk leaves".to_owned());
    }
    Ok((REFERENCE_STORAGE_CLASS.to_owned(), raw_identity))
}

fn observe_os_image() -> Result<String, String> {
    let text = fs::read_to_string("/etc/os-release")
        .map_err(|error| format!("reading /etc/os-release: {error}"))?;
    let mut id = None;
    let mut version = None;
    for line in text.lines() {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim_matches('"');
        match name {
            "ID" => id = Some(value),
            "VERSION_ID" => version = Some(value),
            _ => {}
        }
    }
    let image = format!(
        "{}-{}",
        id.ok_or_else(|| "os-release ID is absent".to_owned())?,
        version.ok_or_else(|| "os-release VERSION_ID is absent".to_owned())?
    );
    if image != REFERENCE_OS_IMAGE {
        return Err(format!(
            "OS image {image:?} differs from required {REFERENCE_OS_IMAGE:?}"
        ));
    }
    Ok(image)
}

fn read_identity_values(paths: &[&str]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !is_placeholder_identity(value))
        .collect()
}

fn privacy_digest(domain: &str, values: &[String]) -> Result<String, String> {
    let mut normalized = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !is_placeholder_identity(value))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    if normalized.is_empty() {
        return Err(format!("{domain} has no usable identity inputs"));
    }
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    for value in normalized {
        hasher.update([0]);
        hasher.update(value.as_bytes());
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn is_placeholder_identity(value: &str) -> bool {
    value.is_empty()
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "none" | "unknown" | "not specified" | "to be filled by o.e.m."
        )
}

fn read_u32(path: &Path) -> Result<u32, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?
        .trim()
        .parse::<u32>()
        .map_err(|error| format!("parsing {}: {error}", path.display()))
}

fn command_output(program: &str, args: &[&str]) -> Result<Output, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("unable to execute {program}: {error}"))?;
    if output.status.success() && output.stderr.is_empty() {
        Ok(output)
    } else {
        Err(format!(
            "{program} failed with {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn stdout_trimmed<'a>(output: &'a Output, probe: &str) -> Result<&'a str, String> {
    std::str::from_utf8(&output.stdout)
        .map(str::trim)
        .map_err(|error| format!("{probe} output is not UTF-8: {error}"))
        .and_then(|value| {
            if value.is_empty() {
                Err(format!("{probe} returned empty output"))
            } else {
                Ok(value)
            }
        })
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> HostAttestationInput {
        HostAttestationInput {
            virtualization: "none".to_owned(),
            physical_cores: 8,
            measurement_cores: REFERENCE_MEASUREMENT_CPUS
                .into_iter()
                .map(|logical_cpu| MeasurementCore {
                    logical_cpu,
                    package_id: 0,
                    core_id: logical_cpu,
                })
                .collect(),
            raw_host_identity: vec!["physical-host-a".to_owned()],
            storage_class: REFERENCE_STORAGE_CLASS.to_owned(),
            raw_storage_identity: vec!["nvme-device-a".to_owned()],
            os_image: REFERENCE_OS_IMAGE.to_owned(),
            toolchain_identity: "rustc-1.94.0".to_owned(),
            prebuild_contract_digest: "a".repeat(64),
        }
    }

    #[test]
    fn raw_host_and_storage_identity_are_hashed_and_never_serialized() {
        let attestation = build_attestation(input()).unwrap();
        let json = serde_json::to_string(&attestation).unwrap();
        assert!(!json.contains("physical-host-a"));
        assert!(!json.contains("nvme-device-a"));
        assert_eq!(attestation.host_digest.len(), 64);
        assert_eq!(attestation.storage_identity_digest.len(), 64);
    }

    #[test]
    fn identity_omission_and_placeholder_values_fail_closed() {
        let mut missing = input();
        missing.raw_host_identity.clear();
        assert!(build_attestation(missing)
            .unwrap_err()
            .contains("no usable"));

        let mut placeholder = input();
        placeholder.raw_storage_identity = vec!["Not Specified".to_owned()];
        assert!(build_attestation(placeholder)
            .unwrap_err()
            .contains("no usable"));
    }

    #[test]
    fn vm_sibling_and_non_nvme_inputs_are_rejected() {
        let mut vm = input();
        vm.virtualization = "kvm".to_owned();
        assert!(build_attestation(vm)
            .unwrap_err()
            .contains("virtualization"));

        let mut sibling = input();
        sibling.measurement_cores[3].core_id = sibling.measurement_cores[2].core_id;
        assert!(build_attestation(sibling).unwrap_err().contains("SMT"));

        let mut storage = input();
        storage.storage_class = "network-block".to_owned();
        assert!(build_attestation(storage).unwrap_err().contains("NVMe"));
    }
}
