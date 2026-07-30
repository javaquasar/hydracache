use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use hydracache_loadgen::profile::{
    CpuIsolationAttestation, MeasurementCore, RunnerAttestationV5,
    REFERENCE_FINGERPRINT_SCHEMA_VERSION, REFERENCE_HOST_CONTRACT_VERSION,
    REFERENCE_HOUSEKEEPING_CPUS, REFERENCE_HOUSEKEEPING_CPU_IDS,
    REFERENCE_HOUSEKEEPING_IDLE_POLICY, REFERENCE_HOUSEKEEPING_MAX_IDLE_LATENCY_US,
    REFERENCE_MEASUREMENT_CPUS, REFERENCE_MEASUREMENT_IDLE_POLICY,
    REFERENCE_MEASUREMENT_MAX_IDLE_LATENCY_US, REFERENCE_OS_IMAGE, REFERENCE_STORAGE_CLASS,
};
use sha2::{Digest, Sha256};

const PROVISIONING_RECEIPT_PATH: &str = "/var/lib/hydracache-perf/runner-provisioned.json";
const MEASUREMENT_IO_POLICY_ENV: &str = "HYDRACACHE_MEASUREMENT_IO_POLICY";
const MEASUREMENT_IO_POLICY: &str = "tmpfs-housekeeping-orchestration-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAttestationInput {
    pub virtualization: String,
    pub physical_cores: u32,
    pub measurement_cores: Vec<MeasurementCore>,
    pub cpu_isolation: CpuIsolationAttestation,
    pub provisioned_host_digest: Option<String>,
    pub raw_host_identity: Vec<String>,
    pub storage_class: String,
    pub raw_storage_identity: Vec<String>,
    pub os_image: String,
    pub toolchain_identity: String,
    pub prebuild_contract_digest: String,
}

pub fn build_attestation(input: HostAttestationInput) -> Result<RunnerAttestationV5, String> {
    let host_digest = match input.provisioned_host_digest {
        Some(digest) => validate_provisioned_host_digest(&digest)?,
        None => privacy_digest("hydracache-host-identity-v2", &input.raw_host_identity)?,
    };
    let storage_identity_digest = privacy_digest(
        "hydracache-storage-identity-v2",
        &input.raw_storage_identity,
    )?;
    let attestation = RunnerAttestationV5 {
        schema_version: REFERENCE_FINGERPRINT_SCHEMA_VERSION,
        contract_version: REFERENCE_HOST_CONTRACT_VERSION.to_owned(),
        virtualization: input.virtualization,
        physical_cores: input.physical_cores,
        measurement_cores: input.measurement_cores,
        cpu_isolation: input.cpu_isolation,
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
) -> Result<RunnerAttestationV5, String> {
    let virtualization = detect_virtualization()?;
    let (physical_cores, measurement_cores) = observe_cpu_topology()?;
    let (provisioned_host_digest, provisioned_cpu_isolation) = read_provisioning_contract()?;
    let cpu_isolation = observe_cpu_isolation(&provisioned_cpu_isolation)?;
    let (storage_class, raw_storage_identity) = observe_root_storage()?;
    let os_image = observe_os_image()?;

    build_attestation(HostAttestationInput {
        virtualization,
        physical_cores,
        measurement_cores,
        cpu_isolation,
        provisioned_host_digest: Some(provisioned_host_digest),
        raw_host_identity: Vec::new(),
        storage_class,
        raw_storage_identity,
        os_image,
        toolchain_identity: toolchain_identity.to_owned(),
        prebuild_contract_digest: prebuild_contract_digest.to_owned(),
    })
}

#[derive(serde::Deserialize)]
struct ProvisioningReceipt {
    cpu_isolation: CpuIsolationAttestation,
    schema_version: u32,
    release: String,
    stage: String,
    source_commit: String,
    host_identity_digest: String,
    runner_online: bool,
    ship_evidence_eligible: bool,
}

fn read_provisioning_contract() -> Result<(String, CpuIsolationAttestation), String> {
    let path = Path::new(PROVISIONING_RECEIPT_PATH);
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("reading provisioning receipt metadata: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err("provisioning receipt is not a regular file".to_owned());
    }
    validate_receipt_metadata(&metadata)?;
    if metadata.len() > 65_536 {
        return Err("provisioning receipt exceeds 64 KiB".to_owned());
    }
    let receipt: ProvisioningReceipt = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("reading provisioning receipt: {error}"))?,
    )
    .map_err(|error| format!("parsing provisioning receipt: {error}"))?;
    let commit = stdout_trimmed(
        &command_output("git", &["rev-parse", "HEAD"])?,
        "git rev-parse HEAD",
    )?
    .to_owned();
    if receipt.schema_version != 4
        || receipt.release != "0.67.1"
        || receipt.stage != "runner-provisioned"
        || receipt.source_commit != commit
        || receipt.runner_online
        || receipt.ship_evidence_eligible
    {
        return Err(
            "provisioning receipt does not match the current qualified host state".to_owned(),
        );
    }
    Ok((
        validate_provisioned_host_digest(&receipt.host_identity_digest)?,
        receipt.cpu_isolation,
    ))
}

fn observe_cpu_isolation(
    provisioned: &CpuIsolationAttestation,
) -> Result<CpuIsolationAttestation, String> {
    let (measurement_idle_policy, measurement_max_idle_latency_us) = observe_idle_policy(
        "measurement",
        &REFERENCE_MEASUREMENT_CPUS,
        REFERENCE_MEASUREMENT_IDLE_POLICY,
        REFERENCE_MEASUREMENT_MAX_IDLE_LATENCY_US,
    )?;
    let (housekeeping_idle_policy, housekeeping_max_idle_latency_us) = observe_idle_policy(
        "housekeeping",
        &REFERENCE_HOUSEKEEPING_CPU_IDS,
        REFERENCE_HOUSEKEEPING_IDLE_POLICY,
        REFERENCE_HOUSEKEEPING_MAX_IDLE_LATENCY_US,
    )?;
    let observed = CpuIsolationAttestation {
        smt_control: read_trimmed(Path::new("/sys/devices/system/cpu/smt/control"))?,
        online_cpus: read_trimmed(Path::new("/sys/devices/system/cpu/online"))?,
        isolated_cpus: read_trimmed(Path::new("/sys/devices/system/cpu/isolated"))?,
        nohz_full_cpus: read_trimmed(Path::new("/sys/devices/system/cpu/nohz_full"))?,
        rcu_nocbs_cpus: kernel_cpu_argument("rcu_nocbs")?,
        housekeeping_cpus: provisioned.housekeeping_cpus.clone(),
        irq_affinity_policy: provisioned.irq_affinity_policy.clone(),
        measurement_idle_policy,
        measurement_max_idle_latency_us,
        housekeeping_idle_policy,
        housekeeping_max_idle_latency_us,
    };
    if &observed != provisioned {
        return Err(format!(
            "runtime CPU isolation differs from the root-owned provisioning receipt: observed={observed:?} provisioned={provisioned:?}"
        ));
    }
    Ok(observed)
}

fn observe_idle_policy(
    role: &str,
    cpus: &[u32],
    policy: &str,
    maximum_latency_us: u32,
) -> Result<(String, u32), String> {
    for cpu in cpus {
        let root = Path::new("/sys/devices/system/cpu")
            .join(format!("cpu{cpu}"))
            .join("cpuidle");
        let mut enabled_shallow = 0_u32;
        let mut disabled_deep = 0_u32;
        for entry in fs::read_dir(&root)
            .map_err(|error| format!("reading {role} CPU {cpu} idle states: {error}"))?
        {
            let state = entry
                .map_err(|error| format!("reading {role} CPU {cpu} idle state: {error}"))?
                .path();
            if !state.is_dir()
                || !state
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("state"))
            {
                continue;
            }
            let latency = read_u32(&state.join("latency"))?;
            let disabled = read_u32(&state.join("disable"))?;
            match (latency <= maximum_latency_us, disabled) {
                (true, 0) => enabled_shallow += 1,
                (false, 1) => disabled_deep += 1,
                (true, _) => {
                    return Err(format!(
                        "{role} CPU {cpu} shallow idle state is disabled: {state:?}"
                    ))
                }
                (false, _) => {
                    return Err(format!(
                        "{role} CPU {cpu} deep idle state is enabled: {state:?}"
                    ))
                }
            }
        }
        if enabled_shallow == 0 || disabled_deep == 0 {
            return Err(format!(
                "{role} CPU {cpu} idle policy did not prove both enabled shallow and disabled deep states"
            ));
        }
    }
    Ok((policy.to_owned(), maximum_latency_us))
}
fn kernel_cpu_argument(name: &str) -> Result<String, String> {
    let cmdline = fs::read_to_string("/proc/cmdline")
        .map_err(|error| format!("reading /proc/cmdline: {error}"))?;
    let prefix = format!("{name}=");
    let matches = cmdline
        .split_whitespace()
        .filter_map(|argument| argument.strip_prefix(&prefix))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [value] if !value.is_empty() => Ok((*value).to_owned()),
        _ => Err(format!(
            "kernel command line must contain exactly one non-empty {name}= argument"
        )),
    }
}

fn validate_receipt_metadata(metadata: &fs::Metadata) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != 0 || metadata.mode() & 0o777 != 0o444 {
            return Err("provisioning receipt must be root-owned with mode 0444".to_owned());
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Err("provisioning receipt attestation requires Unix metadata".to_owned())
    }
}

fn validate_provisioned_host_digest(digest: &str) -> Result<String, String> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(digest.to_owned())
    } else {
        Err("provisioned host identity digest must be lowercase SHA-256".to_owned())
    }
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

fn read_trimmed(path: &Path) -> Result<String, String> {
    let value =
        fs::read_to_string(path).map_err(|error| format!("reading {}: {error}", path.display()))?;
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{} returned empty output", path.display()))
    } else {
        Ok(value.to_owned())
    }
}

fn command_output(program: &str, args: &[&str]) -> Result<Output, String> {
    let policy = std::env::var(MEASUREMENT_IO_POLICY_ENV).ok();
    let (launcher, launcher_args) = probe_command(program, args, policy.as_deref())?;
    let output = Command::new(&launcher)
        .args(&launcher_args)
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

fn probe_command(
    program: &str,
    args: &[&str],
    measurement_io_policy: Option<&str>,
) -> Result<(String, Vec<String>), String> {
    match measurement_io_policy {
        None => Ok((
            program.to_owned(),
            args.iter().map(|argument| (*argument).to_owned()).collect(),
        )),
        Some(MEASUREMENT_IO_POLICY) => {
            let mut launcher_args = vec![
                "--cpu-list".to_owned(),
                REFERENCE_HOUSEKEEPING_CPUS.to_owned(),
                program.to_owned(),
            ];
            launcher_args.extend(args.iter().map(|argument| (*argument).to_owned()));
            Ok(("taskset".to_owned(), launcher_args))
        }
        Some(other) => Err(format!(
            "{MEASUREMENT_IO_POLICY_ENV}={other:?} is not the reviewed reference I/O policy"
        )),
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
            cpu_isolation: CpuIsolationAttestation {
                smt_control: "off".to_owned(),
                online_cpus: "0-7".to_owned(),
                isolated_cpus: "1-4".to_owned(),
                nohz_full_cpus: "1-4".to_owned(),
                rcu_nocbs_cpus: "1-4".to_owned(),
                housekeeping_cpus: "0,5-7".to_owned(),
                irq_affinity_policy: "housekeeping-only-v1".to_owned(),
                measurement_idle_policy: "latency-cap-us-v1".to_owned(),
                measurement_max_idle_latency_us: 1,
                housekeeping_idle_policy: "latency-cap-us-v1".to_owned(),
                housekeeping_max_idle_latency_us: 1,
            },
            provisioned_host_digest: None,
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
    fn protected_provisioning_digest_replaces_raw_host_identity() {
        let mut provisioned = input();
        provisioned.provisioned_host_digest = Some("b".repeat(64));
        provisioned.raw_host_identity.clear();
        assert_eq!(
            build_attestation(provisioned).unwrap().host_digest,
            "b".repeat(64)
        );

        let mut malformed = input();
        malformed.provisioned_host_digest = Some("B".repeat(64));
        assert!(build_attestation(malformed)
            .unwrap_err()
            .contains("lowercase SHA-256"));
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
    #[test]
    fn measurement_host_probes_are_dispatched_on_housekeeping_cpus() {
        let (program, args) =
            probe_command("lsblk", &["--json"], Some(MEASUREMENT_IO_POLICY)).unwrap();
        assert_eq!(program, "taskset");
        assert_eq!(
            args,
            ["--cpu-list", REFERENCE_HOUSEKEEPING_CPUS, "lsblk", "--json"]
        );

        let direct = probe_command("lsblk", &["--json"], None).unwrap();
        assert_eq!(direct, ("lsblk".to_owned(), vec!["--json".to_owned()]));
        assert!(probe_command("lsblk", &[], Some("unreviewed-policy"))
            .unwrap_err()
            .contains("is not the reviewed reference I/O policy"));
    }
}
