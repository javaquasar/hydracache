//! Private nonroot owned-process proof; no production entry or restart writer.
use super::*;
use super::super::{early_attempt, initial_consumption, reservation_publication};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{inspect_empty_fence_at_for_test, with_owned_empty_fence_at_for_test};
use crate::diagnostic_process::{pin_owned_test_helper, OwnedTasksHelper};
use crate::diagnostic_worker_policy::authority_registry::fixture::initialize_fixture_registry;
use crate::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::{open_signed_fixture_credentials, mount_binding::open_signed_fixture_mount};
use crate::host_execution::{lock_host_root, HostExecutionError};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const READY: &str = "HC074-PRIVATE-CRASH-READY ";
const MODE: &str = "HC074_PRIVATE_CRASH_MODE";
const ORIGIN: &str = "HC074_PRIVATE_CRASH_ORIGIN";
const AUTHORITY: &str = "HC074_PRIVATE_CRASH_AUTHORITY";
const ACCOUNT: &str = "HC074_PRIVATE_CRASH_ACCOUNT";
const PHASE: &str = "HC074_PRIVATE_CRASH_PHASE";
const STAGE: &str = "HC074_PRIVATE_CRASH_STAGE";
const EMPTY: &str = "HC074_PRIVATE_CRASH_EMPTY";
const OUTPUT_BUDGET: u64 = 8192;

fn readiness_line(line: &str) -> Option<&str> {
    if !line.ends_with('\n') || line.contains(['\r', '\0']) {
        return None;
    }
    let (prefix, message) = line.split_once(READY)?;
    if !(prefix.is_empty() || prefix.starts_with("test ") && prefix.ends_with(" ... ")) {
        return None;
    }
    let message = message.strip_suffix('\n')?;
    let (label, pid) = message.split_once(' ')?;
    if !label
        .bytes()
        .all(|b| b.is_ascii_alphabetic() || b == b':' || b == b'-')
        || pid.is_empty()
        || !pid.bytes().all(|b| b.is_ascii_digit())
        || pid.parse::<u32>().is_err()
    {
        return None;
    }
    Some(message)
}

#[test]
fn diagnostic_process_crash_ready_parser_accepts_actual_libtest_prefix() {
    for prefix in ["", "test nested::owned_child ... "] {
        let line = format!("{prefix}{READY}consumption:AfterReturn 123\n");
        assert_eq!(readiness_line(&line), Some("consumption:AfterReturn 123"));
    }
    assert_eq!(
        readiness_line(&format!("{READY}fresh-refused 0\n")),
        Some("fresh-refused 0")
    );
}
#[test]
fn diagnostic_process_crash_ready_parser_rejects_malformed_or_ambiguous() {
    for line in [
        format!("junk {READY}reserved:BeforeCreate 123\n"),
        format!("{READY}reserved:BeforeCreate -1\n"),
        format!("{READY}reserved:BeforeCreate 123 extra\n"),
        format!("{READY}reserved:BeforeCreate 4294967296\n"),
        format!("{READY}reserved:BeforeCreate 123\r\n"),
        format!("{READY}reserved:BeforeCreate 123"),
        format!("{READY}reserved:BeforeCreate 123\0\n"),
        format!("{READY}reserved:BeforeCreate 123 {READY}fresh-refused 0\n"),
    ] {
        assert_eq!(readiness_line(&line), None);
    }
}

fn owned_path(variable: &str, prefix: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var_os(variable).unwrap());
    assert_eq!(path.parent(), Some(Path::new("/tmp")));
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with(prefix) && name.len() <= 128);
    assert!(name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
    let m = fs::symlink_metadata(&path).unwrap();
    assert!(m.is_dir() && !m.file_type().is_symlink());
    assert_eq!(
        (m.uid(), m.gid(), m.mode() & 0o7777),
        (
            unsafe { libc::geteuid() },
            unsafe { libc::getegid() },
            0o700
        )
    );
    assert_ne!(m.uid(), 0);
    path
}
fn ready(label: &str, helper: u32, wait: bool) {
    println!("{READY}{label} {helper}");
    std::io::stdout().flush().unwrap();
    if wait {
        let mut command = [0];
        std::io::stdin().read_exact(&mut command).unwrap();
        assert_eq!(command, [b'q']);
    }
}

#[test]
fn diagnostic_process_crash_child() {
    let Some(mode) = std::env::var_os(MODE) else {
        return;
    };
    let origin = owned_path(ORIGIN, "hydracache-fixed-intent-");
    if mode == "inspect" {
        let empty = match std::env::var(EMPTY).unwrap().as_str() {
            "yes" => true,
            "no" => false,
            _ => panic!("invalid private inspect expectation"),
        };
        inspect_empty_fence_at_for_test(&origin, empty);
        ready(
            if empty {
                "fresh-empty"
            } else {
                "fresh-refused"
            },
            0,
            false,
        );
        return;
    }
    assert!(mode == "crash" || mode == "control");
    let authority_root = owned_path(AUTHORITY, "hydracache-authority-");
    let account_root = owned_path(ACCOUNT, "hydracache-crash-account-");
    let phase = std::env::var(PHASE).unwrap();
    let target = std::env::var(STAGE).unwrap();
    assert!(phase == "reserved" || phase == "consumption" || phase == "attempt");
    let permitted = if phase == "attempt" {
        early_attempt::ALL_ATTEMPT_STAGES
            .iter()
            .any(|at| format!("{at:?}") == target)
    } else if phase == "reserved" {
        reservation_publication::ALL_STAGES
            .iter()
            .any(|at| format!("{at:?}") == target)
    } else {
        initial_consumption::ALL_CONSUMPTION_STAGES
            .iter()
            .any(|at| format!("{at:?}") == target)
    };
    assert!(permitted || target == "AfterReturn");
    let owned = OwnedTasksHelper::new(1);
    let process = pin_owned_test_helper(owned.child.id());
    let f = Fixture::in_owned_test_root(&account_root);
    let mut policy = f.checked();
    let mut account = f.account(&mut policy);
    let mut credentials =
        open_signed_fixture_credentials(&mut account, &process, &f.bytes, &f.trust, &f.host)
            .unwrap();
    let mut mount =
        open_signed_fixture_mount(&mut credentials, &f.bytes, &f.trust, &f.host).unwrap();
    let fixture = open_signed_fixture_tasks(&mut mount, &f.bytes, &f.trust, &f.host).unwrap();
    let mut tasks = FixedSignedTasksRead {
        reader: fixture.reader,
    };
    let fixture =
        initialize_fixture_registry(&authority_root, f.policy.uid, f.policy.gid, &f.checked())
            .unwrap();
    let mut registry = modeled_registry_for_test(fixture);
    let mut authority =
        bind_fixed_authority(&mut tasks, &mut registry, &f.bytes, &f.trust, &f.host).unwrap();
    let i = Intent::new(&f);
    let raw = i.signed(&i.body(&f));
    let mut original = bind_fixed_intent(&mut authority, &raw, &i.check(&f, 100)).unwrap();
    with_owned_empty_fence_at_for_test(&origin, |fence, _, _| {
        if phase == "attempt" {
            let mut read =
                early_attempt::retain_with(&mut original, fence, &raw, &i.check(&f, 100), |at| {
                    if format!("{at:?}") == target {
                        ready(&format!("{phase}:{target}"), owned.child.id(), true);
                    }
                    Ok(())
                })
                .unwrap();
            read.revalidate(&raw, &i.check(&f, 100)).unwrap();
            assert_eq!(target, "AfterReturn");
            ready("attempt:AfterReturn", owned.child.id(), true);
            return;
        }
        let mut read = reservation_publication::publish_with(
            &mut original,
            fence,
            &raw,
            &i.check(&f, 100),
            |at| {
                if phase == "reserved" && format!("{at:?}") == target {
                    ready(&format!("{phase}:{target}"), owned.child.id(), true);
                }
                Ok(())
            },
        )
        .unwrap();
        if phase == "reserved" {
            read.revalidate(&raw, &i.check(&f, 100)).unwrap();
            assert_eq!(target, "AfterReturn");
            ready("reserved:AfterReturn", owned.child.id(), true);
        } else {
            let mut read = initial_consumption::consume_with(read, &raw, &i.check(&f, 100), |at| {
                if format!("{at:?}") == target {
                    ready(&format!("{phase}:{target}"), owned.child.id(), true);
                }
                Ok(())
            })
            .unwrap();
            read.revalidate(&raw, &i.check(&f, 100)).unwrap();
            assert_eq!(target, "AfterReturn");
            ready("consumption:AfterReturn", owned.child.id(), true);
        }
    });
}

struct Scratch {
    origin: tempfile::TempDir,
    authority: tempfile::TempDir,
    account: tempfile::TempDir,
}
impl Scratch {
    fn new() -> Self {
        let dir = |prefix| {
            let dir = tempfile::Builder::new()
                .prefix(prefix)
                .tempdir_in("/tmp")
                .unwrap();
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
            dir
        };
        Self {
            origin: dir("hydracache-fixed-intent-"),
            authority: dir("hydracache-authority-"),
            account: dir("hydracache-crash-account-"),
        }
    }
    fn leaf(&self) -> PathBuf {
        self.origin
            .path()
            .join("var/lib/hydracache-performance/diagnostic-intents-074")
    }
    fn campaigns(&self) -> PathBuf {
        self.origin
            .path()
            .join("var/lib/hydracache-performance/campaigns")
    }
}
fn terminal(fd: &OwnedFd, milliseconds: i32) -> bool {
    let mut poll = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&mut poll, 1, milliseconds) };
    assert!(result >= 0);
    assert_eq!(poll.revents & (libc::POLLERR | libc::POLLNVAL), 0);
    result == 1 && poll.revents & libc::POLLIN != 0
}
type Output = BufReader<std::io::Take<std::process::ChildStdout>>;
struct OwnedSubprocess {
    child: Child,
    helper: Option<OwnedFd>,
    collector: Option<JoinHandle<()>>,
    output: Option<Output>,
}
impl OwnedSubprocess {
    fn start(s: &Scratch, mode: &str, phase: &str, stage: &str, empty: bool) -> (Self, String) {
        let full = format!("{}::diagnostic_process_crash_child", module_path!());
        let (_, test) = full.split_once("::").unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .env(MODE, mode)
            .env(ORIGIN, s.origin.path())
            .env(AUTHORITY, s.authority.path())
            .env(ACCOUNT, s.account.path())
            .env(PHASE, phase)
            .env(STAGE, stage)
            .env(EMPTY, if empty { "yes" } else { "no" })
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut owned = Self {
            child,
            helper: None,
            collector: None,
            output: None,
        };
        let output = owned.child.stdout.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        owned.collector = Some(thread::spawn(move || {
            let mut reader = BufReader::new(output.take(OUTPUT_BUDGET));
            loop {
                let mut line = String::new();
                let count = reader.read_line(&mut line).unwrap();
                if count == 0 {
                    let _ = send.send((None, reader));
                    break;
                }
                if let Some(message) = readiness_line(&line) {
                    let _ = send.send((Some(message.to_owned()), reader));
                    break;
                }
            }
        }));
        // A harness timeout is a plumbing failure, never a passing correctness verdict.
        let (message, reader) = receive
            .recv_timeout(Duration::from_secs(60))
            .expect("owned child did not reach its logical checkpoint");
        owned.output = Some(reader);
        owned.collector.take().unwrap().join().unwrap();
        (
            owned,
            message.expect("owned child exited without a checkpoint"),
        )
    }
    fn pin_helper(&mut self, pid: u32) {
        let status = fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
        let parent = status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:\t"))
            .unwrap()
            .parse::<u32>()
            .unwrap();
        assert_eq!(parent, self.child.id());
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        assert!(fd >= 0, "failed to pin exact owned helper");
        self.helper = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        assert!(!terminal(self.helper.as_ref().unwrap(), 0));
    }
    fn finish(&mut self, kill: bool) {
        if kill {
            self.child.kill().unwrap();
        } else {
            self.child.stdin.as_mut().unwrap().write_all(b"q").unwrap();
        }
        let status = self.child.wait().unwrap();
        if kill {
            assert_eq!(status.signal(), Some(libc::SIGKILL));
        } else {
            assert!(status.success(), "uninterrupted control failed: {status}");
        }
        if let Some(helper) = &self.helper {
            assert!(
                terminal(helper, 5000),
                "owned helper survived original owner's death/drop"
            );
        }
        let mut tail = Vec::new();
        self.output
            .as_mut()
            .unwrap()
            .read_to_end(&mut tail)
            .unwrap();
        assert!(tail.len() < OUTPUT_BUDGET as usize);
    }
}
impl Drop for OwnedSubprocess {
    fn drop(&mut self) {
        if self.child.try_wait().is_ok_and(|status| status.is_none()) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        if let Some(helper) = &self.helper {
            if !terminal(helper, 0) {
                // Cleanup targets the retained pidfd, never a reused numeric PID or process group.
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        helper.as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0,
                    );
                }
                let _ = terminal(helper, 5000);
            }
        }
        if let Some(collector) = self.collector.take() {
            let _ = collector.join();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    name: String,
    metadata: [u64; 11],
    bytes: Vec<u8>,
}
fn snapshot(s: &Scratch) -> Vec<Entry> {
    let mut entries = vec![];
    for (label, directory) in [
        ("intent", s.leaf()),
        ("authority", s.authority.path().to_owned()),
        ("campaign", s.campaigns()),
    ] {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let m = fs::symlink_metadata(&path).unwrap();
            assert!(m.is_file() && !m.file_type().is_symlink() && m.len() <= 65536);
            entries.push(Entry {
                name: format!("{label}/{}", path.file_name().unwrap().to_str().unwrap()),
                metadata: [
                    m.dev(),
                    m.ino(),
                    u64::from(m.uid()),
                    u64::from(m.gid()),
                    u64::from(m.mode()),
                    m.nlink(),
                    m.len(),
                    m.mtime() as u64,
                    m.mtime_nsec() as u64,
                    m.ctime() as u64,
                    m.ctime_nsec() as u64,
                ],
                bytes: fs::read(path).unwrap(),
            });
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}
fn restart_inspect(s: &Scratch, empty: bool) {
    let (mut child, message) = OwnedSubprocess::start(s, "inspect", "", "", empty);
    assert_eq!(
        message,
        if empty {
            "fresh-empty 0"
        } else {
            "fresh-refused 0"
        }
    );
    assert!(child.child.wait().unwrap().success());
}
fn checkpoint_contents(s: &Scratch, phase: &str, stage: &str) {
    let names: Vec<_> = fs::read_dir(s.leaf())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    if phase == "attempt" {
        assert_eq!(names.len(), usize::from(stage != "BeforeCreate"));
        assert!(!s.leaf().join("active-diagnostic.json").exists());
        if stage == "BeforeCreate" {
            return;
        }
        let path = s.leaf().join("diagnostic-attempt-074.json");
        let raw = fs::read(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().mode() & 0o7777,
            if matches!(stage, "Created" | "PrefixWritten" | "BytesWritten") {
                0o600
            } else {
                0o400
            }
        );
        match stage {
            "Created" => assert!(raw.is_empty()),
            "PrefixWritten" => {
                assert!(!raw.is_empty());
                assert!(serde_json::from_slice::<serde_json::Value>(&raw).is_err());
            }
            _ => {
                let value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                assert_eq!(value["schema_version"], "diagnostic-retained-intent-074-v1");
                assert!(value["previous_record_sha256"].is_null());
                assert_eq!(value["state"]["revision"], 1);
                assert_eq!(value["state"]["completed_cells"], 0);
                assert_eq!(value["state"]["stage"], "reserved");
                assert_eq!(value["state"]["promotable"], false);
                assert_eq!(value["state"]["admission_allowed"], false);
                let mut canonical = crate::canonical_json(&value).unwrap();
                canonical.push(b'\n');
                assert_eq!(raw, canonical);
            }
        }
        return;
    }
    if phase == "reserved" {
        assert_eq!(names.len(), usize::from(stage != "BeforeCreate"));
        if stage == "BeforeCreate" {
            return;
        }
        let raw = fs::read(s.leaf().join("active-diagnostic.json")).unwrap();
        match stage {
            "StateCreated" => assert!(raw.is_empty()),
            "PrefixWritten" => {
                assert!(!raw.is_empty());
                assert!(crate::diagnostic_lease::decode_active(&raw).is_err());
            }
            _ => {
                let state = crate::diagnostic_lease::decode_active(&raw).unwrap();
                assert!(!state.promotable && !state.admission_allowed);
            }
        }
        return;
    }
    let index = initial_consumption::ALL_CONSUMPTION_STAGES
        .iter()
        .position(|at| format!("{at:?}") == stage)
        .unwrap_or_else(|| {
            assert_eq!(stage, "AfterReturn");
            initial_consumption::ALL_CONSUMPTION_STAGES.len()
        });
    assert_eq!(
        names.len(),
        if index == 0 {
            1
        } else if index < 7 {
            2
        } else {
            3
        }
    );
    if index == 0 {
        return;
    }
    let record_path = s.leaf().join("diagnostic-intent-cell-1.json");
    let record = fs::read(&record_path).unwrap();
    assert_eq!(
        fs::metadata(record_path).unwrap().mode() & 0o7777,
        if index >= 4 { 0o400 } else { 0o600 }
    );
    match index {
        1 => assert!(record.is_empty()),
        2 => {
            assert!(!record.is_empty());
            assert!(serde_json::from_slice::<serde_json::Value>(&record).is_err());
        }
        _ => {
            let decoded: serde_json::Value = serde_json::from_slice(&record).unwrap();
            assert_eq!(
                decoded["schema_version"],
                "diagnostic-retained-intent-074-v1"
            );
            assert!(decoded["previous_record_sha256"].is_null());
            assert_eq!(record.last(), Some(&b'\n'));
        }
    }
    if index < 7 {
        return;
    }
    let path = s.leaf().join("diagnostic-intent-cell-1.commit");
    let marker = fs::read(&path).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().mode() & 0o7777,
        if index >= 10 { 0o400 } else { 0o600 }
    );
    match index {
        7 => assert!(marker.is_empty()),
        8 => {
            assert!(!marker.is_empty() && marker.len() < 65);
        }
        _ => assert_eq!(
            marker,
            format!("{}\n", crate::sha256_hex(&record)).as_bytes()
        ),
    }
}
fn run_case(phase: &str, stage: &str, kill: bool) {
    let s = Scratch::new();
    let (mut child, message) = OwnedSubprocess::start(
        &s,
        if kill { "crash" } else { "control" },
        phase,
        stage,
        false,
    );
    let (label, helper) = message.rsplit_once(' ').unwrap();
    assert_eq!(label, format!("{phase}:{stage}"));
    child.pin_helper(helper.parse().unwrap());
    assert!(matches!(
        lock_host_root(&s.campaigns()),
        Err(HostExecutionError::Busy)
    ));
    let original_snapshot = snapshot(&s);
    checkpoint_contents(&s, phase, stage);
    let state = s.leaf().join("active-diagnostic.json");
    let empty = matches!(phase, "reserved" | "attempt") && stage == "BeforeCreate";
    assert_eq!(state.exists(), phase != "attempt" && !empty);
    if phase == "consumption" || (phase != "attempt" && stage == "AfterReturn") {
        let state = crate::diagnostic_lease::decode_active(&fs::read(&state).unwrap()).unwrap();
        assert_eq!(state.stage, DiagnosticStage::Reserved);
        assert_eq!((state.revision, state.completed_cells), (1, 0));
        assert!(!state.promotable && !state.admission_allowed);
    }
    child.finish(kill);
    drop(lock_host_root(&s.campaigns()).expect("original flock survived terminated owner"));
    assert_eq!(snapshot(&s), original_snapshot);
    for _ in 0..2 {
        restart_inspect(&s, empty);
        let restarted_snapshot = snapshot(&s);
        assert_eq!(restarted_snapshot, original_snapshot);
    }
    eprintln!("{} case={phase}:{stage} terminated={} signal={} fresh_processes=2 retained=unchanged original_lock=released helper=terminal", if phase == "attempt" { "P1" } else { "P0" }, if kill { "SIGKILL" } else { "control" }, if kill { 9 } else { 0 });
}
fn seeded(mut stages: Vec<String>) -> Vec<String> {
    let mut seed = 0x074d_u64;
    for index in (1..stages.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        stages.swap(index, seed as usize % (index + 1));
    }
    eprintln!("P0 seed=0x074d stages={}", stages.len());
    stages
}
#[test]
#[ignore = "manual local nonroot Linux owned SIGKILL proof, no product workload"]
fn diagnostic_process_crash_reserved_all_stages_and_post_return() {
    let mut stages: Vec<_> = reservation_publication::ALL_STAGES
        .iter()
        .map(|at| format!("{at:?}"))
        .collect();
    stages.push("AfterReturn".into());
    for stage in seeded(stages) {
        run_case("reserved", &stage, true);
    }
}
#[test]
#[ignore = "manual local nonroot Linux owned SIGKILL proof, no product workload"]
fn diagnostic_process_crash_consumption_all_stages_and_post_return() {
    let mut stages: Vec<_> = initial_consumption::ALL_CONSUMPTION_STAGES
        .iter()
        .map(|at| format!("{at:?}"))
        .collect();
    stages.push("AfterReturn".into());
    for stage in seeded(stages) {
        run_case("consumption", &stage, true);
    }
}
#[test]
#[ignore = "manual local nonroot Linux uninterrupted control, no product workload"]
fn diagnostic_process_crash_uninterrupted_controls_keep_context() {
    for phase in ["reserved", "consumption"] {
        run_case(phase, "AfterReturn", false);
    }
}

#[test]
#[ignore = "manual local nonroot Linux owned early-attempt SIGKILL proof, no product workload"]
fn diagnostic_process_crash_attempt_all_stages_and_post_return() {
    let mut stages: Vec<_> = early_attempt::ALL_ATTEMPT_STAGES
        .iter()
        .map(|at| format!("{at:?}"))
        .collect();
    stages.push("AfterReturn".into());
    for stage in seeded(stages) {
        run_case("attempt", &stage, true);
    }
    run_case("attempt", "AfterReturn", false);
}
