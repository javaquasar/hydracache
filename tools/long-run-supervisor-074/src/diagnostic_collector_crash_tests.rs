//! Private actual process interruption at original collector barriers.
use super::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const MODE: &str = "HC074_PRIVATE_COLLECTOR_MODE";
const ROOT: &str = "HC074_PRIVATE_COLLECTOR_ROOT";
const TARGET: &str = "HC074_PRIVATE_COLLECTOR_STAGE";
const READY: &str = "HC074-COLLECTOR-READY ";
const CHILD: &str = "diagnostic_collector::crash_tests::collector_crash_child";
const STAGES: [Stage; 14] = [
    Stage::DirectoryCreated,
    Stage::Directory,
    Stage::StdoutCreated,
    Stage::StdoutSynced,
    Stage::StderrCreated,
    Stage::StderrSynced,
    Stage::Write,
    Stage::StdoutSync,
    Stage::StderrSync,
    Stage::Readonly,
    Stage::ReadonlySynced,
    Stage::DirectorySync,
    Stage::Close,
    Stage::Acknowledge,
];

pub(super) fn pause_at(stage: Stage) {
    if std::env::var(MODE).ok().as_deref() == Some("crash")
        && std::env::var(TARGET).ok().as_deref() == Some(format!("{stage:?}").as_str())
    {
        pause(&format!("{stage:?}"));
    }
}
fn pause(stage: &str) {
    println!("{READY}{stage} {}", std::process::id());
    std::io::stdout().flush().unwrap();
    let mut release = [0];
    std::io::stdin().read_exact(&mut release).unwrap();
    assert_eq!(release, [b'q']);
}
fn owned_root() -> PathBuf {
    let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
    assert_eq!(root.parent(), Some(Path::new("/tmp")));
    assert!(root
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hydracache-collector-crash-"));
    open_fixture_root(&root).unwrap();
    root
}
#[test]
fn collector_crash_child() {
    let Some(mode) = std::env::var_os(MODE) else {
        return;
    };
    let root = owned_root();
    if mode == "inspect" {
        assert!(matches!(
            prepare_fixture(&root, 256, None),
            Err(CollectorError::Conflict)
        ));
        return;
    }
    assert!(mode == "crash" || mode == "control");
    let (mut c, Writers([mut stdout, mut stderr])) = prepare_fixture(&root, 256, None).unwrap();
    stdout.write_all(b"owned stdout\0\xff").unwrap();
    stderr.write_all(b"owned stderr\0\xff").unwrap();
    drop(stdout);
    drop(stderr);
    while !c.pump().unwrap().both_eof() {}
    let receipt = c.finish().unwrap();
    assert_eq!(receipt.bytes, [14, 14]);
    if mode == "crash" {
        assert_eq!(std::env::var(TARGET).unwrap(), "AfterReturn");
        pause("AfterReturn");
    }
}
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn spawn(root: &Path, mode: &str, stage: &str) -> Owned {
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--nocapture", "--test-threads=1"])
        .env(MODE, mode)
        .env(ROOT, root)
        .env(TARGET, stage)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    Owned(child)
}
fn wait(child: &mut Owned) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline, "owned collector helper stalled");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn barrier(child: &mut Owned, stage: &str) -> File {
    let stdout = child.0.stdout.take().unwrap();
    let descriptor: std::os::fd::OwnedFd = stdout.into();
    let mut pipe = File::from(descriptor);
    assert_eq!(
        unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    let target = format!("{READY}{stage} {}\n", child.0.id());
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut retained = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => panic!(
                "owned collector helper exited before {stage}: {}",
                String::from_utf8_lossy(&retained)
            ),
            Ok(count) => retained.extend_from_slice(&buffer[..count]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
            Err(e) => panic!("readiness pipe failed: {e}"),
        }
        assert!(retained.len() <= 8192, "helper output overflow");
        if retained
            .windows(target.len())
            .any(|v| v == target.as_bytes())
        {
            return pipe;
        }
        assert!(
            Instant::now() < deadline,
            "owned collector barrier stalled at {stage}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn snapshot(root: &Path) -> std::collections::BTreeMap<String, (Stamp, Vec<u8>)> {
    let mut result = std::collections::BTreeMap::new();
    let attempt = root.join(ATTEMPT);
    let m = fs::symlink_metadata(&attempt).unwrap();
    assert!(m.is_dir() && !m.file_type().is_symlink());
    result.insert(ATTEMPT.into(), (Stamp::of(&m), Vec::new()));
    for entry in fs::read_dir(attempt).unwrap() {
        let entry = entry.unwrap();
        let m = fs::symlink_metadata(entry.path()).unwrap();
        assert!(m.is_file() && !m.file_type().is_symlink() && m.len() <= 256);
        result.insert(
            entry.file_name().into_string().unwrap(),
            (Stamp::of(&m), fs::read(entry.path()).unwrap()),
        );
    }
    assert!(result.len() <= 3);
    result
}
#[test]
#[ignore = "manual nonroot Linux actual owned collector SIGKILL; single-flight; no live service"]
fn collector_owned_sigkill_matrix_retains_without_replay() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let stages: Vec<_> = STAGES
        .into_iter()
        .map(|s| format!("{s:?}"))
        .chain(std::iter::once("AfterReturn".into()))
        .collect();
    for stage in &stages {
        let root = tempfile::Builder::new()
            .prefix("hydracache-collector-crash-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut child = spawn(root.path(), "crash", stage);
        let _pipe = barrier(&mut child, stage);
        let before = snapshot(root.path());
        child.0.kill().unwrap();
        assert_eq!(wait(&mut child).signal(), Some(libc::SIGKILL));
        assert_eq!(snapshot(root.path()), before);
        for _ in 0..2 {
            let mut inspect = spawn(root.path(), "inspect", stage);
            assert!(wait(&mut inspect).success());
            assert_eq!(snapshot(root.path()), before);
        }
        eprintln!("seed=0x0750 checkpoint={stage} actual_sigkill=1 fresh_refusals=2 unchanged_retention=true");
    }
    let root = tempfile::Builder::new()
        .prefix("hydracache-collector-crash-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let mut child = spawn(root.path(), "control", "Control");
    assert!(wait(&mut child).success());
    let before = snapshot(root.path());
    for _ in 0..2 {
        let mut inspect = spawn(root.path(), "inspect", "Control");
        assert!(wait(&mut inspect).success());
        assert_eq!(snapshot(root.path()), before);
    }
    eprintln!("seed=0x0750 sigkill_cases=15 controls=1 fresh_refusals=32 all_owned_helpers_reaped=true production_authority=false");
}
