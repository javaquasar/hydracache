use crate::ProcessIdentity;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

const PROC_DOCUMENT_LIMIT: u64 = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    pub boot_id: String,
    pub pid: u32,
    pub start_ticks: u64,
    pub process_group: i64,
    pub cgroup_path: String,
    pub cgroup_inode: u64,
    pub cpus_allowed_list: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMismatch {
    BootId,
    StartTicks,
    ProcessGroup,
    CgroupPath,
    CgroupInode,
    UnitName,
    CpuSet,
}

#[derive(Debug, Error)]
pub enum ProcessIdentityError {
    #[error("process identity input or proc document is malformed")]
    Document,
    #[error("process identity I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("live process identity differs: {0:?}")]
    Mismatch(Vec<IdentityMismatch>),
}

pub fn inspect_process(pid: u32) -> Result<ProcessSnapshot, ProcessIdentityError> {
    if pid == 0 {
        return Err(ProcessIdentityError::Document);
    }
    let stat = read_bounded(&PathBuf::from(format!("/proc/{pid}/stat")))?;
    let (stat_pid, process_group, start_ticks) = parse_stat(&stat)?;
    if stat_pid != pid {
        return Err(ProcessIdentityError::Document);
    }
    let cgroup = read_bounded(&PathBuf::from(format!("/proc/{pid}/cgroup")))?;
    let cgroup_path = parse_unified_cgroup(&cgroup)?;
    let relative = cgroup_path
        .strip_prefix('/')
        .ok_or(ProcessIdentityError::Document)?;
    let cgroup_directory = Path::new("/sys/fs/cgroup").join(relative);
    let metadata = fs::metadata(cgroup_directory)?;
    if !metadata.is_dir() {
        return Err(ProcessIdentityError::Document);
    }
    let boot_id = fs::read_to_string("/proc/sys/kernel/random/boot_id")?
        .trim()
        .to_owned();
    if boot_id.is_empty() || boot_id.len() > 128 {
        return Err(ProcessIdentityError::Document);
    }
    let status = read_bounded(&PathBuf::from(format!("/proc/{pid}/status")))?;
    let cpus_allowed_list = parse_status_value(&status, "Cpus_allowed_list")?;
    Ok(ProcessSnapshot {
        boot_id,
        pid,
        start_ticks,
        process_group,
        cgroup_path,
        cgroup_inode: metadata.ino(),
        cpus_allowed_list,
    })
}

pub fn verify_process_cpuset(
    expected: &ProcessIdentity,
    expected_cpuset: &str,
) -> Result<(), ProcessIdentityError> {
    if expected_cpuset.is_empty() || expected_cpuset.len() > 256 {
        return Err(ProcessIdentityError::Document);
    }
    let current = inspect_process(expected.pid)?;
    if current.cpus_allowed_list == expected_cpuset {
        Ok(())
    } else {
        Err(ProcessIdentityError::Mismatch(vec![
            IdentityMismatch::CpuSet,
        ]))
    }
}

pub fn verify_process_identity(expected: &ProcessIdentity) -> Result<(), ProcessIdentityError> {
    let current = inspect_process(expected.pid)?;
    let mut mismatches = Vec::new();
    if current.boot_id != expected.boot_id {
        mismatches.push(IdentityMismatch::BootId);
    }
    if current.start_ticks != expected.start_ticks {
        mismatches.push(IdentityMismatch::StartTicks);
    }
    if current.process_group != expected.process_group {
        mismatches.push(IdentityMismatch::ProcessGroup);
    }
    if current.cgroup_path != expected.cgroup_path {
        mismatches.push(IdentityMismatch::CgroupPath);
    }
    if current.cgroup_inode != expected.cgroup_inode {
        mismatches.push(IdentityMismatch::CgroupInode);
    }
    if expected.unit_name.is_empty()
        || Path::new(&current.cgroup_path)
            .components()
            .next_back()
            .and_then(|component| match component {
                Component::Normal(value) => value.to_str(),
                _ => None,
            })
            != Some(expected.unit_name.as_str())
    {
        mismatches.push(IdentityMismatch::UnitName);
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(ProcessIdentityError::Mismatch(mismatches))
    }
}

fn read_bounded(path: &Path) -> Result<String, ProcessIdentityError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > PROC_DOCUMENT_LIMIT {
        return Err(ProcessIdentityError::Document);
    }
    let value = fs::read_to_string(path)?;
    if value.is_empty() || value.len() as u64 > PROC_DOCUMENT_LIMIT {
        return Err(ProcessIdentityError::Document);
    }
    Ok(value)
}

fn parse_stat(value: &str) -> Result<(u32, i64, u64), ProcessIdentityError> {
    let open = value.find('(').ok_or(ProcessIdentityError::Document)?;
    let close = value.rfind(')').ok_or(ProcessIdentityError::Document)?;
    if close <= open || value[..open].trim().is_empty() {
        return Err(ProcessIdentityError::Document);
    }
    let pid = value[..open]
        .trim()
        .parse::<u32>()
        .map_err(|_| ProcessIdentityError::Document)?;
    let fields = value[close + 1..]
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if fields.len() <= 19 || fields[0].len() != 1 {
        return Err(ProcessIdentityError::Document);
    }
    let process_group = fields[2]
        .parse::<i64>()
        .map_err(|_| ProcessIdentityError::Document)?;
    let start_ticks = fields[19]
        .parse::<u64>()
        .map_err(|_| ProcessIdentityError::Document)?;
    Ok((pid, process_group, start_ticks))
}

fn parse_unified_cgroup(value: &str) -> Result<String, ProcessIdentityError> {
    let rows = value.lines().collect::<Vec<_>>();
    if rows.len() != 1 {
        return Err(ProcessIdentityError::Document);
    }
    let path = rows[0]
        .strip_prefix("0::")
        .ok_or(ProcessIdentityError::Document)?;
    let parsed = Path::new(path);
    if !parsed.is_absolute()
        || parsed
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        || path.contains('\0')
        || path.contains('\n')
        || path.contains('\r')
    {
        return Err(ProcessIdentityError::Document);
    }
    Ok(path.to_owned())
}

fn parse_status_value(value: &str, key: &str) -> Result<String, ProcessIdentityError> {
    let prefix = format!("{key}:");
    let mut matches = value.lines().filter_map(|line| line.strip_prefix(&prefix));
    let result = matches
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or(ProcessIdentityError::Document)?;
    if matches.next().is_some() {
        return Err(ProcessIdentityError::Document);
    }
    Ok(result.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{parse_stat, parse_status_value, parse_unified_cgroup};

    #[test]
    fn stat_parser_handles_spaces_and_closing_parentheses_in_comm() {
        let mut fields = vec!["0"; 20];
        fields[0] = "S";
        fields[2] = "77";
        fields[19] = "999";
        let stat = format!("123 (worker ) name) {}", fields.join(" "));
        assert_eq!(parse_stat(&stat).unwrap(), (123, 77, 999));
    }

    #[test]
    fn cgroup_parser_accepts_only_one_unified_absolute_path() {
        assert_eq!(
            parse_unified_cgroup("0::/system.slice/example.service\n").unwrap(),
            "/system.slice/example.service"
        );
        assert!(parse_unified_cgroup("1:name=/legacy\n").is_err());
        assert!(parse_unified_cgroup("0::/a\n0::/b\n").is_err());
        assert!(parse_unified_cgroup("0::/../escape\n").is_err());
    }

    #[test]
    fn status_parser_requires_one_nonempty_bounded_value() {
        assert_eq!(
            parse_status_value(
                "Name:\ttest\nCpus_allowed_list:\t2-7\n",
                "Cpus_allowed_list"
            )
            .unwrap(),
            "2-7"
        );
        assert!(parse_status_value("Name:\ttest\n", "Cpus_allowed_list").is_err());
        assert!(parse_status_value(
            "Cpus_allowed_list:\t2-7\nCpus_allowed_list:\t0-1\n",
            "Cpus_allowed_list"
        )
        .is_err());
    }
}
