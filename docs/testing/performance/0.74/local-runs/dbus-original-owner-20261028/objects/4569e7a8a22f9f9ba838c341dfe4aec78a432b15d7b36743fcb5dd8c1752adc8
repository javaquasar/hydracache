use super::*;
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn terminal_process_poll_masks_exhaustive_fail_closed() {
    for bits in 0..=u16::MAX {
        let mask = bits as i16;
        match classify(1, mask) {
            Ok(TerminalState::Exited) => assert_eq!(mask, libc::POLLIN),
            Ok(TerminalState::Reaped) => {
                assert!(mask == libc::POLLHUP || mask == libc::POLLIN | libc::POLLHUP)
            }
            Err(ProcessError::Invalid) => {
                assert!(![libc::POLLIN, libc::POLLHUP, libc::POLLIN | libc::POLLHUP].contains(&mask))
            }
            _ => panic!("unexpected terminal classification"),
        }
    }
    for count in [-1, 0, 2, i32::MAX] {
        for mask in [0, libc::POLLIN, libc::POLLHUP, libc::POLLIN | libc::POLLHUP] {
            assert!(classify(count, mask).is_err());
        }
    }
    println!("terminal-mask exhaustive=65536 accepted=3 unknown_or_inconsistent=refused");
}
struct Fake {
    calls: RefCell<Vec<&'static str>>,
    fail: Option<usize>,
    ready: (i32, i16),
    drift_after: bool,
}
impl Fake {
    fn step(&self, name: &'static str) -> Result<(), ProcessError> {
        self.calls.borrow_mut().push(name);
        if self.fail == Some(self.calls.borrow().len()) {
            return Err(std::io::Error::from_raw_os_error(libc::EIO).into());
        }
        if self.drift_after && name == "origin" && self.calls.borrow().len() > 1 {
            return Err(ProcessError::Drift);
        }
        Ok(())
    }
}
impl TerminalProbe for Fake {
    fn origin(&self) -> Result<(), ProcessError> {
        self.step("origin")
    }
    fn poll(&self) -> Result<(i32, i16), ProcessError> {
        self.step("poll")?;
        Ok(self.ready)
    }
    fn live_generation(&self) -> Result<(), ProcessError> {
        self.step("live")
    }
}
#[test]
fn terminal_process_sequence_preserves_first_error_without_late_reads() {
    for ready in [
        (0, 0),
        (1, libc::POLLIN),
        (1, libc::POLLHUP),
        (1, libc::POLLIN | libc::POLLHUP),
    ] {
        let order = if ready == (0, 0) {
            vec!["origin", "poll", "live", "origin"]
        } else {
            vec!["origin", "poll", "origin"]
        };
        let fake = Fake {
            calls: RefCell::new(vec![]),
            fail: None,
            ready,
            drift_after: false,
        };
        observe(&fake).unwrap();
        assert_eq!(*fake.calls.borrow(), order);
        for at in 1..=order.len() {
            let fake = Fake {
                calls: RefCell::new(vec![]),
                fail: Some(at),
                ready,
                drift_after: false,
            };
            assert!(
                matches!(observe(&fake), Err(ProcessError::Io(ref e)) if e.raw_os_error() == Some(libc::EIO))
            );
            assert_eq!(*fake.calls.borrow(), order[..at]);
        }
    }
}
#[test]
fn terminal_process_late_boot_drift_and_unknown_poll_never_become_exit() {
    for ready in [(0, 0), (1, libc::POLLIN), (1, libc::POLLHUP)] {
        let fake = Fake {
            calls: RefCell::new(vec![]),
            fail: None,
            ready,
            drift_after: true,
        };
        assert!(matches!(observe(&fake), Err(ProcessError::Drift)));
    }
    for ready in [
        (0, libc::POLLIN),
        (1, 0),
        (1, libc::POLLIN | libc::POLLERR),
        (2, libc::POLLIN),
        (1, libc::POLLNVAL),
    ] {
        let fake = Fake {
            calls: RefCell::new(vec![]),
            fail: None,
            ready,
            drift_after: false,
        };
        assert!(matches!(observe(&fake), Err(ProcessError::Invalid)));
        assert_eq!(*fake.calls.borrow(), ["origin", "poll"]);
    }
}
#[test]
fn terminal_process_no_event_requires_original_live_generation() {
    struct Dead(Cell<usize>);
    impl TerminalProbe for Dead {
        fn origin(&self) -> Result<(), ProcessError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
        fn poll(&self) -> Result<(i32, i16), ProcessError> {
            self.0.set(self.0.get() + 1);
            Ok((0, 0))
        }
        fn live_generation(&self) -> Result<(), ProcessError> {
            self.0.set(self.0.get() + 1);
            Err(ProcessError::NotLive)
        }
    }
    let dead = Dead(Cell::new(0));
    assert!(matches!(observe(&dead), Err(ProcessError::NotLive)));
    assert_eq!(dead.0.get(), 3);
}
struct Owned {
    child: Child,
    control: Option<ChildStdin>,
    original: ProcessRead,
    descendant: Option<ProcessRead>,
}
fn signal_original(read: &ProcessRead) -> std::io::Result<()> {
    // SAFETY: retained pidfd from this fixture's original owned process; no numeric PID signal.
    let n = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            read.probe.pidfd.as_raw_fd(),
            libc::SIGKILL,
            std::ptr::null::<libc::siginfo_t>(),
            0u32,
        )
    };
    if n < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
// A child-supplied numeric PID is not ownership. Check the kernel parent under
// both retained generation brackets before retaining any descendant for signals.
fn is_original_child(child: &ProcessRead, parent: &ProcessRead) -> Result<bool, ProcessError> {
    parent.revalidate()?;
    child.revalidate()?;
    let status = super::super::Document::open(&child.probe.files.directory, "status")?;
    let raw = status.read(&child.probe.files.directory)?;
    let text = std::str::from_utf8(&raw).map_err(|_| ProcessError::Invalid)?;
    let mut fields = text.lines().filter_map(|line| line.strip_prefix("PPid:\t"));
    let ppid = fields.next().ok_or(ProcessError::Invalid)?;
    let ppid =
        u32::try_from(super::super::number(ppid.as_bytes())?).map_err(|_| ProcessError::Invalid)?;
    if fields.next().is_some() {
        return Err(ProcessError::Invalid);
    }
    child.revalidate()?;
    parent.revalidate()?;
    Ok(ppid == parent.observation().expected().pid)
}
impl Owned {
    fn cat() -> Self {
        let mut child = Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let original = super::super::pin_owned_test_helper(child.id());
        let control = child.stdin.take();
        Self {
            child,
            control,
            original,
            descendant: None,
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.control.take();
        if let Some(descendant) = &self.descendant {
            let _ = signal_original(descendant);
        }
        let _ = signal_original(&self.original);
        let _ = self.child.wait();
    }
}
fn wait_terminal(read: &ProcessRead) {
    let mut fd = libc::pollfd {
        fd: read.probe.pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one retained descriptor, bounded harness wait only.
    assert_eq!(unsafe { libc::poll(&mut fd, 1, 30_000) }, 1);
    assert!(read.observe_terminal().unwrap().is_some());
}
#[test]
fn terminal_process_foreign_owned_generation_is_not_adopted_as_descendant() {
    let first = Owned::cat();
    let other = Owned::cat();
    assert!(!is_original_child(&other.original, &first.original).unwrap());
    first.original.revalidate().unwrap();
    other.original.revalidate().unwrap();
}
#[test]
fn terminal_process_actual_graceful_exit_reap_and_replacement_stay_separate() {
    let mut owned = Owned::cat();
    let original = owned.original.observation().clone();
    assert!(owned.original.observe_terminal().unwrap().is_none());
    owned.control.take();
    wait_terminal(&owned.original);
    let before_reap = owned.original.observe_terminal().unwrap().unwrap();
    assert_eq!(before_reap.state(), TerminalState::Exited);
    assert_eq!(before_reap.generation(), &original);
    assert!(owned.child.wait().unwrap().success());
    let after = owned.original.observe_terminal().unwrap().unwrap();
    // Owner wait proves reaping independently. Older kernels can continue
    // reporting readable exit without HUP; neither notification proves cleanup.
    assert!(matches!(
        after.state(),
        TerminalState::Exited | TerminalState::Reaped
    ));
    assert_eq!(after.generation(), &original);
    assert!(matches!(
        owned.original.revalidate(),
        Err(ProcessError::NotLive)
    ));
    let replacement = Owned::cat();
    assert!(replacement.original.observe_terminal().unwrap().is_none());
    assert_ne!(
        replacement.original.observation().expected().pid,
        original.expected().pid
    );
    assert_eq!(
        owned
            .original
            .observe_terminal()
            .unwrap()
            .unwrap()
            .generation(),
        &original
    );
}
#[test]
fn terminal_process_actual_sigkill_reports_readiness_not_successful_outcome() {
    let mut owned = Owned::cat();
    let original = owned.original.observation().clone();
    signal_original(&owned.original).unwrap();
    assert_eq!(owned.child.wait().unwrap().signal(), Some(9));
    let observed = owned.original.observe_terminal().unwrap().unwrap();
    assert!(matches!(
        observed.state(),
        TerminalState::Exited | TerminalState::Reaped
    ));
    assert_eq!(observed.generation(), &original);
    assert!(matches!(
        owned.original.revalidate(),
        Err(ProcessError::NotLive)
    ));
}
// Whole-line deadline and length bound: even a partial reply cannot hang the fixture.
fn line(stdout: &mut ChildStdout) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut bytes = vec![];
    for _ in 0..64 {
        let left = deadline
            .checked_duration_since(Instant::now())
            .expect("fixture reply deadline");
        let timeout = i32::try_from(left.as_millis()).unwrap().max(1);
        let mut fd = libc::pollfd {
            fd: stdout.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: owned stdout, one initialized poll descriptor.
        assert_eq!(unsafe { libc::poll(&mut fd, 1, timeout) }, 1);
        let mut byte = [0];
        stdout.read_exact(&mut byte).unwrap();
        if byte == [b'\n'] {
            return String::from_utf8(bytes).unwrap();
        }
        bytes.push(byte[0]);
    }
    panic!("fixture reply exceeded line bound");
}
#[test]
#[ignore = "manual local nonroot owned leader/descendant exit proof; no product workload or cgroup authority"]
fn terminal_process_owned_leader_exit_does_not_stop_descendant_writer() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    // Test-only fixed script: descendant control EOF terminates both processes on early failure.
    const PYTHON: &str = r#"import os
child = os.fork()
if child == 0:
    while os.read(0, 1):
        os.write(1, b"writer-after-leader-exit\n")
    os._exit(0)
os.write(1, ("descendant:" + str(child) + "\n").encode())
os.waitpid(child, 0)
"#;
    let mut child = Command::new("python3")
        .args(["-I", "-S", "-c", PYTHON])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let original = super::super::pin_owned_test_helper(child.id());
    let control = child.stdin.take();
    let mut owned = Owned {
        child,
        control,
        original,
        descendant: None,
    };
    let header = line(owned.child.stdout.as_mut().unwrap());
    let pid = header
        .strip_prefix("descendant:")
        .unwrap()
        .parse::<u32>()
        .unwrap();
    assert_ne!(pid, owned.child.id());
    let descendant = super::super::pin_owned_test_helper(pid);
    assert!(is_original_child(&descendant, &owned.original).unwrap());
    // An unverified PID is never stored in the RAII signal cleanup set.
    owned.descendant = Some(descendant);
    let original = owned.original.observation().clone();
    signal_original(&owned.original).unwrap();
    assert_eq!(owned.child.wait().unwrap().signal(), Some(9));
    assert_eq!(
        owned
            .original
            .observe_terminal()
            .unwrap()
            .unwrap()
            .generation(),
        &original
    );
    let descendant = owned.descendant.as_ref().unwrap();
    descendant.revalidate().unwrap();
    assert!(descendant.observe_terminal().unwrap().is_none());
    owned.control.as_mut().unwrap().write_all(b"w").unwrap();
    assert_eq!(
        line(owned.child.stdout.as_mut().unwrap()),
        "writer-after-leader-exit"
    );
    descendant.revalidate().unwrap();
    assert!(descendant.observe_terminal().unwrap().is_none());
    owned.control.take();
    wait_terminal(descendant);
    println!("P1 terminal seed=0x074f leader=SIGKILL signal=9 original_pidfd=terminal descendant=live post_exit_write=observed descendant_after_control_eof=terminal completion_authority=absent");
}
