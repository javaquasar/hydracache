//! Private original collector Starting SIGKILL proof, never an installed entry point.
use super::*;
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{
    inspect_empty_fence_at_for_test, with_owned_empty_fence_at_for_test,
};
use crate::host_execution::{lock_host_root, HostExecutionError, HOST_EXECUTION_LOCK_NAME};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const READY: &str = "HC074-COLLECTOR-STARTING-READY ";
const ROOT: &str = "HC074_PRIVATE_COLLECTOR_STARTING_CRASH_ROOT";
const MODE: &str = "HC074_PRIVATE_COLLECTOR_STARTING_CRASH_MODE";
const STAGE: &str = "HC074_PRIVATE_COLLECTOR_STARTING_CRASH_STAGE";
const AUTHORITY: &str = "HC074_PRIVATE_COLLECTOR_STARTING_CRASH_AUTHORITY";
const BUDGET: u64 = 8192;
const SEED: u64 = 0x0740_57d1;

fn valid_stage(stage: &str) -> bool {
    stage == "AfterReturn" || STAGES.iter().any(|at| format!("{at:?}") == stage)
}
fn readiness_line(line: &str) -> Option<(&str, u32)> {
    if !line.ends_with('\n') || line.contains(['\r', '\0']) {
        return None;
    }
    let (prefix, body) = line.split_once(READY)?;
    if !(prefix.is_empty() || prefix.starts_with("test ") && prefix.ends_with(" ... ")) {
        return None;
    }
    let (stage, pid) = body.strip_suffix('\n')?.split_once(' ')?;
    if !(valid_stage(stage) || matches!(stage, "fresh-refused"))
        || pid.is_empty()
        || !pid.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let pid = pid.parse().ok()?;
    (pid > 0).then_some((stage, pid))
}

#[test]
fn collector_starting_process_protocol_accepts_only_bounded_expected_records() {
    for prefix in ["", "test nested::owned_child ... "] {
        for stage in [
            "BeforeCreate",
            "Acknowledging",
            "AfterReturn",
            "fresh-refused",
        ] {
            let line = format!("{prefix}{READY}{stage} 123\n");
            assert_eq!(readiness_line(&line), Some((stage, 123)));
        }
    }
    for line in [
        format!("junk {READY}AfterReturn 123\n"),
        format!("{READY}AfterReturn 0\n"),
        format!("{READY}AfterReturn -1\n"),
        format!("{READY}AfterReturn 4294967296\n"),
        format!("{READY}AfterReturn 123 extra\n"),
        format!("{READY}AfterReturn 123\r\n"),
        format!("{READY}AfterReturn 123\0\n"),
        format!("{READY}AfterReturn 123"),
        format!("{READY}unknown 123\n"),
        format!("{READY}AfterReturn 123 {READY}fresh-empty 123\n"),
    ] {
        assert_eq!(readiness_line(&line), None, "{line:?}");
    }
}

fn ready(stage: &str, wait: bool) {
    println!("{READY}{stage} {}", std::process::id());
    std::io::stdout().flush().unwrap();
    if wait {
        let mut command = [0];
        std::io::stdin().read_exact(&mut command).unwrap();
        assert_eq!(command, [b'q']);
    }
}

fn owned_root() -> PathBuf {
    let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
    assert_eq!(root.parent(), Some(Path::new("/tmp")));
    let name = root.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("hydracache-collector-starting-crash-") && name.len() <= 128);
    assert!(name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'));
    assert_eq!(fs::canonicalize(&root).unwrap(), root);
    for path in [&root, &root.join("context"), &root.join("origin")] {
        let m = fs::symlink_metadata(path).unwrap();
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
    }
    assert_eq!(std::env::temp_dir(), root.join("context"));
    root
}

#[test]
fn collector_starting_process_child() {
    let Some(mode) = std::env::var_os(MODE) else {
        return;
    };
    let root = owned_root();
    let stage = std::env::var(STAGE).unwrap();
    assert!(valid_stage(&stage));
    if mode == "inspect" {
        inspect_empty_fence_at_for_test(&root.join("origin"), false);
        ready("fresh-refused", false);
        return;
    }
    assert!(mode == "kill" || mode == "control");
    let authority = PathBuf::from(std::env::var_os(AUTHORITY).unwrap());
    assert_eq!(authority.parent(), Some(Path::new("/tmp")));
    assert_eq!(fs::canonicalize(&authority).unwrap(), authority);
    assert!(authority
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hydracache-authority-"));
    let m = fs::symlink_metadata(&authority).unwrap();
    assert!(m.is_dir() && !m.file_type().is_symlink());
    assert_eq!(
        (m.uid(), m.gid(), m.mode() & 0o7777),
        (
            unsafe { libc::geteuid() },
            unsafe { libc::getegid() },
            0o700
        )
    );
    with_original_at(
        false,
        Some(&authority),
        |original, _, receiver, sender, model| {
            with_owned_empty_fence_at_for_test(&root.join("origin"), |fence, _, leaf| {
                let framed = accepted(original, fence, leaf, receiver, sender, model);
                let mut visited = Vec::new();
                let mut owner = framed
                    .publish_starting_with(|at| {
                        visited.push(at);
                        assert_eq!(visited, STAGES[..visited.len()]);
                        if format!("{at:?}") == stage {
                            ready(&stage, true);
                        }
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(visited, STAGES);
                assert_eq!(stage, "AfterReturn");
                owner.revalidate().unwrap();
                ready("AfterReturn", true);
                owner.revalidate().unwrap();
            });
        },
    );
}

struct Scratch(tempfile::TempDir, tempfile::TempDir);
impl Scratch {
    fn new() -> Self {
        assert_ne!(
            unsafe { libc::geteuid() },
            0,
            "manual proof requires nonroot"
        );
        let root = tempfile::Builder::new()
            .prefix("hydracache-collector-starting-crash-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        for name in ["context", "origin"] {
            fs::create_dir(root.path().join(name)).unwrap();
            fs::set_permissions(root.path().join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        let authority = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(authority.path(), fs::Permissions::from_mode(0o700)).unwrap();
        Self(root, authority)
    }
    fn origin(&self) -> PathBuf {
        self.0.path().join("origin")
    }
    fn context(&self) -> PathBuf {
        self.0.path().join("context")
    }
    fn leaf(&self) -> PathBuf {
        self.origin()
            .join("var/lib/hydracache-performance/diagnostic-intents-074")
    }
    fn campaigns(&self) -> PathBuf {
        self.origin()
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
struct OwnedChild {
    child: Child,
    pidfd: Option<OwnedFd>,
    collector: Option<JoinHandle<()>>,
    output: Option<Output>,
}
impl OwnedChild {
    fn start(s: &Scratch, mode: &str, stage: &str) -> Self {
        let full = format!("{}::collector_starting_process_child", module_path!());
        let (_, test) = full.split_once("::").unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .env(ROOT, s.0.path())
            .env(AUTHORITY, s.1.path())
            .env(MODE, mode)
            .env(STAGE, stage)
            .env("TMPDIR", s.context())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut owned = Self {
            child,
            pidfd: None,
            collector: None,
            output: None,
        };
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, owned.child.id(), 0) };
        assert!(fd >= 0, "pin exact owned child before waiting");
        owned.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        let output = owned.child.stdout.take().unwrap();
        let (send, receive) = mpsc::sync_channel(1);
        owned.collector = Some(thread::spawn(move || {
            let mut reader = BufReader::new(output.take(BUDGET));
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 {
                    let _ = send.send((None, reader));
                    break;
                }
                if let Some((label, pid)) = readiness_line(&line) {
                    let _ = send.send((Some((label.to_owned(), pid)), reader));
                    break;
                }
            }
        }));
        // This is a harness fail-safe, never the correctness oracle.
        let (message, reader) = receive
            .recv_timeout(Duration::from_secs(60))
            .expect("owned original chain did not reach logical checkpoint");
        owned.output = Some(reader);
        owned.collector.take().unwrap().join().unwrap();
        let expected = if mode == "inspect" {
            "fresh-refused"
        } else {
            stage
        };
        assert_eq!(message, Some((expected.to_owned(), owned.child.id())));
        owned
    }
    fn finish(&mut self, kill: bool, inspection: bool) {
        if kill {
            // The descriptor pins this exact unreaped child, not a discovered PID.
            assert_eq!(
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        self.pidfd.as_ref().unwrap().as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0,
                    )
                },
                0
            );
        } else if !inspection {
            self.child.stdin.as_mut().unwrap().write_all(b"q").unwrap();
        }
        assert!(
            terminal(self.pidfd.as_ref().unwrap(), 60_000),
            "owned child did not terminate"
        );
        let status = self.child.wait().unwrap();
        if kill {
            assert_eq!(status.signal(), Some(libc::SIGKILL));
        } else {
            assert!(status.success(), "owned child failed: {status}");
        }
        let mut tail = Vec::new();
        self.output
            .as_mut()
            .unwrap()
            .read_to_end(&mut tail)
            .unwrap();
        assert!(tail.len() < BUDGET as usize);
        assert!(
            self.output.as_ref().unwrap().get_ref().limit() > 0,
            "child output budget exhausted"
        );
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.child.try_wait().is_ok_and(|status| status.is_none()) {
            if let Some(fd) = &self.pidfd {
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        fd.as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0,
                    );
                }
            } else {
                let _ = self.child.kill();
            }
        }
        let _ = self.child.wait();
        if let Some(collector) = self.collector.take() {
            let _ = collector.join();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    name: PathBuf,
    metadata: [u64; 11],
    bytes: Vec<u8>,
}
fn snapshot(root: &Path) -> Vec<Entry> {
    fn visit(root: &Path, path: &Path, entries: &mut Vec<Entry>) {
        let m = fs::symlink_metadata(path).unwrap();
        assert!(!m.file_type().is_symlink() && (m.is_dir() || m.is_file()));
        assert_eq!(
            (m.uid(), m.gid()),
            (unsafe { libc::geteuid() }, unsafe { libc::getegid() })
        );
        assert!(m.len() <= 65_536);
        if m.is_file() {
            assert_eq!(m.nlink(), 1);
        }
        entries.push(Entry {
            name: path.strip_prefix(root).unwrap().to_owned(),
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
            bytes: if m.is_file() {
                fs::read(path).unwrap()
            } else {
                vec![]
            },
        });
        if m.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), entries);
            }
        }
    }
    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

fn controller_checkpoint_contents(
    s: &Scratch,
    stage: &str,
    context: &[Entry],
    authority: &[Entry],
) {
    assert_eq!(fs::read_dir(s.campaigns()).unwrap().count(), 1);
    assert!(s.campaigns().join(HOST_EXECUTION_LOCK_NAME).is_file());
    if stage == "BeforeCreate" {
        return;
    }
    let path = s.leaf().join(NAME);
    let raw = fs::read(&path).unwrap();
    assert!(raw.len() <= 16_384);
    assert_eq!(
        fs::metadata(path).unwrap().mode() & 0o7777,
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
            assert!(serde_json::from_slice::<Value>(&raw).is_err());
        }
        _ => {
            validate_retained_controller_attempt(&raw).unwrap();
            let value: Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(value["policy_epoch"], 7);
            assert_eq!(value["authority_revision"], 1);
            let records: Vec<_> = authority
                .iter()
                .filter(|e| e.name == Path::new("0000000000000001.json"))
                .collect();
            assert_eq!(records.len(), 1);
            let anchor: Value = serde_json::from_slice(&records[0].bytes).unwrap();
            assert_eq!(anchor["revision"], value["authority_revision"]);
            assert_eq!(anchor["policy_epoch"], value["policy_epoch"]);
            assert_eq!(anchor["policy_sha256"], value["worker_policy_sha256"]);
            assert_eq!(anchor["state"], "active");
            assert_eq!(authority.iter().filter(|e| !e.bytes.is_empty()).count(), 2);
            assert_eq!(value["build_receipt_sha256"], crate::sha256_hex(RECEIPT));
            assert_eq!(
                value["builder_policy_sha256"],
                crate::sha256_hex(BUILDER_POLICY)
            );
            for (field, filename) in [
                ("worker_trust_sha256", "worker-trust-074.json"),
                ("controller_trust_sha256", "controller-trust-074.json"),
                ("oidc_trust_sha256", "oidc-trust-074.json"),
            ] {
                let matches: Vec<_> = context
                    .iter()
                    .filter(|e| e.name.file_name().is_some_and(|n| n == filename))
                    .collect();
                assert_eq!(matches.len(), 1, "{filename}");
                assert_eq!(value[field], crate::sha256_hex(&matches[0].bytes));
            }
            assert!(value.get("token").is_none() && value.get("signature_hex").is_none());
        }
    }
}

fn checkpoint_contents(s: &Scratch, stage: &str, context: &[Entry], authority: &[Entry]) {
    controller_checkpoint_contents(s, "AfterReturn", context, authority);
    let request = fs::read(s.leaf().join(NAME)).unwrap();
    let reserved = fs::read(s.leaf().join(RESERVED_NAME)).unwrap();
    validate_retained_collector_reserved(&reserved, &request).unwrap();
    assert_eq!(
        fs::metadata(s.leaf().join(RESERVED_NAME)).unwrap().mode() & 0o7777,
        0o400
    );
    assert_eq!(
        fs::read_dir(s.leaf()).unwrap().count(),
        if stage == "BeforeCreate" { 2 } else { 3 }
    );
    let path = s.leaf().join(COLLECTOR_STARTING_NAME);
    if stage == "BeforeCreate" {
        assert!(!path.exists());
        return;
    }
    let raw = fs::read(&path).unwrap();
    assert!(raw.len() <= MAX_STARTING_BYTES);
    assert_eq!(
        fs::metadata(path).unwrap().mode() & 0o7777,
        if matches!(
            stage,
            "Created" | "CreateDirectorySynced" | "PrefixWritten" | "BytesWritten"
        ) {
            0o600
        } else {
            0o400
        }
    );
    match stage {
        "Created" | "CreateDirectorySynced" => assert!(raw.is_empty()),
        "PrefixWritten" => {
            assert!(!raw.is_empty());
            assert!(validate_retained_collector_starting(&raw, &request, &reserved).is_err());
        }
        _ => {
            validate_retained_collector_starting(&raw, &request, &reserved).unwrap();
        }
    }
}

fn run_case(stage: &str, kill: bool, repeat: u32) {
    let s = Scratch::new();
    let mut child = OwnedChild::start(&s, if kill { "kill" } else { "control" }, stage);
    assert!(!terminal(child.pidfd.as_ref().unwrap(), 0));
    assert!(matches!(
        lock_host_root(&s.campaigns()),
        Err(HostExecutionError::Busy)
    ));
    let origin = snapshot(&s.origin());
    let context = snapshot(&s.context());
    let authority = snapshot(s.1.path());
    checkpoint_contents(&s, stage, &context, &authority);
    child.finish(kill, false);
    drop(lock_host_root(&s.campaigns()).expect("original flock survived terminated owner"));
    assert_eq!(snapshot(&s.origin()), origin);
    assert_eq!(snapshot(s.1.path()), authority);
    if kill {
        assert_eq!(snapshot(&s.context()), context);
    } else {
        assert_eq!(fs::read_dir(s.context()).unwrap().count(), 0);
    }
    let retained_context = snapshot(&s.context());
    for inspection in 1..=2 {
        let mut fresh = OwnedChild::start(&s, "inspect", stage);
        fresh.finish(false, true);
        assert_eq!(snapshot(&s.origin()), origin);
        assert_eq!(snapshot(&s.context()), retained_context);
        assert_eq!(snapshot(s.1.path()), authority);
        println!("collector_starting_crash seed=0x{SEED:08x} repeat={repeat} stage={stage} sigkill={kill} fresh_inspection={inspection} empty=false unchanged=true");
    }
}

fn run_collector_crash_matrix(repeat: u32) {
    let mut order: Vec<_> = STAGES.iter().map(|stage| format!("{stage:?}")).collect();
    order.push("AfterReturn".into());
    let mut seed = SEED;
    for i in (1..order.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        order.swap(i, seed as usize % (i + 1));
    }
    for stage in order {
        run_case(&stage, true, repeat);
    }
    run_case("AfterReturn", false, repeat);
    println!("collector_starting_crash_summary seed=0x{SEED:08x} repeat={repeat} sigkills=10 controls=1 fresh_inspections=22 production_enrollment=false start=false");
}

#[test]
#[ignore = "manual nonroot Linux original collector Starting SIGKILL proof"]
fn collector_starting_process_crash_all_stages_and_lost_ack() {
    for repeat in 1..=3 {
        run_collector_crash_matrix(repeat);
    }
}
