//! Ordinary test isolation only; no runtime owner, lease, service or release authority.
use std::fs::File;
use std::io::{self, Read, Seek};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CAPTURE_BYTES: u64 = 1_048_576;
const CASE_BOUND: Duration = Duration::from_secs(45);
const CLEANUP_BOUND: Duration = Duration::from_secs(5);

pub fn case_name(module: &str, leaf: &str) -> String {
    match module.split_once("::") {
        Some((_, suffix)) => format!("{suffix}::{leaf}"),
        None => leaf.to_owned(),
    }
}
pub fn selected_alone(name: &str, args: &[String]) -> bool {
    args.len() == 4 && args[1] == "--exact" && args[2] == name && args[3] == "--test-threads=1"
}
pub fn is_child_case(module: &str, leaf: &str) -> bool {
    selected_alone(
        &case_name(module, leaf),
        &std::env::args().collect::<Vec<_>>(),
    )
}
pub fn one_success(name: &str, stdout: &str) -> bool {
    stdout
        .lines()
        .filter(|line| *line == "running 1 test")
        .count()
        == 1
        && stdout
            .lines()
            .filter(|line| *line == format!("test {name} ... ok"))
            .count()
            == 1
        && stdout
            .lines()
            .filter(|line| line.starts_with("test result:"))
            .count()
            == 1
        && stdout.lines().any(|line| {
            line.strip_prefix("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; ")
                .and_then(|suffix| suffix.split_once(" filtered out; finished in "))
                .is_some_and(|(filtered, elapsed)| {
                    filtered.parse::<usize>().is_ok() && elapsed.ends_with('s')
                })
        })
        && !stdout.contains("running 0 tests")
        && !stdout.contains(" ... FAILED")
}
#[derive(Debug)]
pub struct Outcome {
    pub success: bool,
    pub fault: Option<&'static str>,
    pub stdout: String,
    pub stderr: String,
}
struct Owned {
    child: Child,
    reaped: bool,
}
impl Owned {
    fn stop_group(&mut self) -> io::Result<()> {
        // The successful process_group(0) child remains unreaped throughout.
        // Its numerical PID/PGID cannot be recycled before our exact wait.
        let pid = i32::try_from(self.child.id()).map_err(io::Error::other)?;
        if pid <= 0 {
            return Err(io::Error::other("invalid owned process group"));
        }
        let result = unsafe { libc::kill(-pid, libc::SIGKILL) };
        if result != 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(io::Error::last_os_error());
        }
        // Also stop the exact owned leader if a faulty case escaped its group.
        self.child.kill().or_else(|error| {
            if error.kind() == io::ErrorKind::InvalidInput {
                Ok(())
            } else {
                Err(error)
            }
        })
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.stop_group();
            let _ = self.child.wait();
        }
    }
}
fn terminal_without_reap(pid: u32) -> io::Result<bool> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { info.si_pid() } == pid as i32)
}
fn live_group_members(group: u32) -> io::Result<Vec<u32>> {
    let mut live = Vec::new();
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let raw = match std::fs::read_to_string(entry.path().join("stat")) {
            Ok(raw) => raw,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let fields: Vec<_> = raw
            .rsplit_once(')')
            .ok_or_else(|| io::Error::other("invalid proc stat"))?
            .1
            .split_whitespace()
            .collect();
        let pgid = fields
            .get(2)
            .ok_or_else(|| io::Error::other("short proc stat"))?
            .parse::<u32>()
            .map_err(io::Error::other)?;
        if pgid == group && !matches!(fields.first(), Some(&"Z" | &"X")) {
            live.push(pid);
        }
    }
    Ok(live)
}
fn capture(file: &mut File) -> io::Result<String> {
    if file.metadata()?.len() > CAPTURE_BYTES {
        return Err(io::Error::other("owned case capture budget"));
    }
    file.rewind()?;
    let mut raw = String::new();
    file.take(CAPTURE_BYTES + 1).read_to_string(&mut raw)?;
    if raw.len() as u64 > CAPTURE_BYTES {
        return Err(io::Error::other("owned case capture grew"));
    }
    Ok(raw)
}
pub fn run_case(name: &str, bound: Duration) -> io::Result<Outcome> {
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    if unsafe { libc::sigaction(libc::SIGCHLD, std::ptr::null(), &mut action) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if action.sa_sigaction != libc::SIG_DFL || action.sa_flags & libc::SA_NOCLDWAIT != 0 {
        return Err(io::Error::other(
            "owned test requires default explicit child reaping",
        ));
    }
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let child = Command::new(std::env::current_exe()?)
        .args(["--exact", name, "--test-threads=1"])
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?)
        .process_group(0)
        .spawn()?;
    let mut owned = Owned {
        child,
        reaped: false,
    };
    let deadline = Instant::now() + bound;
    let timed_out = loop {
        if terminal_without_reap(owned.child.id())? {
            break false;
        }
        if Instant::now() >= deadline {
            break true;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let stray = live_group_members(owned.child.id())?
        .into_iter()
        .any(|pid| pid != owned.child.id());
    if timed_out || stray {
        owned.stop_group()?;
        let cleanup = Instant::now() + CLEANUP_BOUND;
        while !live_group_members(owned.child.id())?.is_empty() {
            if Instant::now() >= cleanup {
                return Err(io::Error::other("owned group remains live after SIGKILL"));
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let status = owned.child.wait()?;
    owned.reaped = true;
    let stdout = capture(&mut stdout)?;
    let stderr = capture(&mut stderr)?;
    let fault = if timed_out {
        Some("timeout-cleanup-is-not-success")
    } else if stray {
        Some("unexpected-live-descendant")
    } else if !status.success() {
        Some("child-failed")
    } else if !one_success(name, &stdout) {
        Some("child-selection-or-result")
    } else {
        None
    };
    Ok(Outcome {
        success: fault.is_none(),
        fault,
        stdout,
        stderr,
    })
}
pub fn isolate(module: &str, leaf: &str) -> bool {
    if is_child_case(module, leaf) {
        return false;
    }
    let name = case_name(module, leaf);
    let outcome = run_case(&name, CASE_BOUND).expect("owned test process failed to retire");
    assert!(
        outcome.success,
        "owned case {name}: {:?}\n{}\n{}",
        outcome.fault, outcome.stdout, outcome.stderr
    );
    true
}
