//! Owned nonexecuting process cuts; never systemd delivery or writer-tree proof.
use super::*;
const GROUP_MODE: &str = "HC074_PRIVATE_FIXED_GROUPED_PREPARATION_CRASH_MODE";
const GROUP_STAGE: &str = "HC074_PRIVATE_FIXED_GROUPED_PREPARATION_CRASH_STAGE";
const GROUP_READY: &str = "HC074-FIXED-GROUPED-PROCESS-READY ";
const ACK: &str = "HC074-FIXED-GROUPED-RETURN-ACK ";
const GROUP_SEED: u64 = 0x0740_6c17;
const POINTS: [&str; 6] = [
    "opened",
    "loaded",
    "bound",
    "message",
    "armed",
    "returned-unacknowledged",
];

fn group_line(line: &str) -> Option<(&str, u32)> {
    if line.len() > BUDGET as usize || !line.ends_with('\n') || line.contains(['\r', '\0']) {
        return None;
    }
    let (prefix, body) = line.split_once(GROUP_READY)?;
    if !(prefix.is_empty() || prefix.starts_with("test ") && prefix.ends_with(" ... ")) {
        return None;
    }
    let (stage, pid) = body.strip_suffix('\n')?.split_once(' ')?;
    if !(POINTS.contains(&stage) || stage == "fresh-refused")
        || pid.is_empty()
        || !pid.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let pid = pid.parse().ok()?;
    (pid > 0).then_some((stage, pid))
}
fn acknowledgements(tail: &[u8], pid: u32) -> Result<u8, ()> {
    let text = std::str::from_utf8(tail).map_err(|_| ())?;
    let mut count = 0;
    for line in text.split_inclusive('\n').filter(|l| l.contains(ACK)) {
        if line != format!("{ACK}{pid}\n") {
            return Err(());
        }
        count += 1;
        if count > 1 {
            return Err(());
        }
    }
    Ok(count)
}
fn group_ready(stage: &str, wait: bool) {
    println!("{GROUP_READY}{stage} {}", std::process::id());
    std::io::stdout().flush().unwrap();
    if wait {
        let mut command = [0];
        std::io::stdin().read_exact(&mut command).unwrap();
        assert_eq!(command, [b'q']);
    }
}
fn order() -> Vec<&'static str> {
    let mut points = POINTS.to_vec();
    let mut seed = GROUP_SEED;
    for i in (1..points.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        points.swap(i, seed as usize % (i + 1));
    }
    points
}
#[test]
fn fixed_grouped_process_protocol_rejects_invalid_or_ambiguous_records() {
    for prefix in ["", "test nested::owned_child ... "] {
        for point in POINTS.into_iter().chain(["fresh-refused"]) {
            assert_eq!(
                group_line(&format!("{prefix}{GROUP_READY}{point} 123\n")),
                Some((point, 123))
            );
        }
    }
    for line in [
        format!("junk {GROUP_READY}opened 123\n"),
        format!("{GROUP_READY}unknown 123\n"),
        format!("{GROUP_READY}armed 0\n"),
        format!("{GROUP_READY}armed -1\n"),
        format!("{GROUP_READY}armed 4294967296\n"),
        format!("{GROUP_READY}armed 123 extra\n"),
        format!("{GROUP_READY}armed 123\r\n"),
        format!("{GROUP_READY}armed 123\0\n"),
        format!("{GROUP_READY}armed 123"),
        format!("{GROUP_READY}armed 123 {GROUP_READY}armed 123\n"),
        format!("{}{GROUP_READY}armed 123\n", "x".repeat(BUDGET as usize)),
    ] {
        assert_eq!(group_line(&line), None, "{line:?}");
    }
    let mut sorted = order();
    sorted.sort_unstable();
    let mut expected = POINTS.to_vec();
    expected.sort_unstable();
    assert_eq!(sorted, expected);
    assert_eq!(order(), order());
    println!("HC074-FIXED-GROUPED-PROTOCOL seed=0x{GROUP_SEED:08x} points=6 exact=true");
}
#[test]
fn fixed_grouped_process_ack_codec_distinguishes_absent_exact_and_duplicate() {
    assert_eq!(acknowledgements(b"ok\n", 123), Ok(0));
    assert_eq!(
        acknowledgements(format!("{ACK}123\nok\n").as_bytes(), 123),
        Ok(1)
    );
    for raw in [
        format!("{ACK}123\n{ACK}123\n"),
        format!("{ACK}124\n"),
        format!("junk {ACK}123\n"),
        format!("{ACK}123"),
        format!("{ACK}123\r\n"),
    ] {
        assert_eq!(acknowledgements(raw.as_bytes(), 123), Err(()));
    }
    assert_eq!(acknowledgements(&[255], 123), Err(()));
}

#[test]
fn original_fixed_grouped_process_child() {
    let Some(mode) = std::env::var_os(GROUP_MODE) else {
        return;
    };
    let root = owned_root();
    let stage = std::env::var(GROUP_STAGE).unwrap();
    assert!(POINTS.contains(&stage.as_str()));
    if mode == "inspect" {
        inspect_empty_fence_at_for_test(&root.join("origin"), false);
        group_ready("fresh-refused", false);
        return;
    }
    assert!(mode == "kill" || mode == "control" || mode == "ack-lost");
    let authority = PathBuf::from(std::env::var_os(AUTHORITY).unwrap());
    assert_eq!(authority.parent(), Some(Path::new("/tmp")));
    assert_eq!(fs::canonicalize(&authority).unwrap(), authority);
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
    assert!(authority
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("hydracache-authority-"));
    let bundle = PathBuf::from(std::env::var_os("HC074_START_MATERIAL_BUNDLE").unwrap());
    assert!(bundle.is_absolute());
    assert_ne!(bundle, Path::new(crate::diagnostic_artifacts::INSTALL_ROOT));
    with_original_at(
        false,
        Some(&authority),
        |original, _, receiver, sender, model| {
            with_owned_empty_fence_at_for_test(&root.join("origin"), |fence, _, leaf| {
                let outputs = accepted(original, fence, leaf, receiver, sender, model)
                    .prepare_original_outputs_for_test(|_| Ok(()))
                    .unwrap();
                let mut visited = Vec::new();
                let mut alarm = outputs
                    .prepare_fixed_grouped_fixture_for_test(&bundle, |at, _| {
                        visited.push(at);
                        if at == stage {
                            group_ready(at, true);
                        }
                    })
                    .unwrap();
                assert_eq!(visited, POINTS[..5]);
                assert_eq!(stage, "returned-unacknowledged");
                assert_eq!(
                    alarm.offsets_for_test(),
                    [300_000_000_000, 60_000_000_000, 10_000_000_000]
                );
                group_ready("returned-unacknowledged", true);
                alarm.revalidate_fixture_for_test().unwrap();
                assert!(!alarm.is_refused());
                drop(alarm);
                if mode == "control" {
                    println!("{ACK}{}", std::process::id());
                    std::io::stdout().flush().unwrap();
                }
            });
        },
    );
}

impl OwnedChild {
    fn fixed_grouped_start(s: &Scratch, mode: &str, stage: &str) -> Self {
        let full = format!("{}::original_fixed_grouped_process_child", module_path!());
        let (_, test) = full.split_once("::").unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture", "--test-threads=1"])
            .env(ROOT, s.0.path())
            .env(AUTHORITY, s.1.path())
            .env(GROUP_MODE, mode)
            .env(GROUP_STAGE, stage)
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
                if let Some((label, pid)) = group_line(&line) {
                    let _ = send.send((Some((label.to_owned(), pid)), reader));
                    break;
                }
            }
        }));
        let (message, reader) = receive
            .recv_timeout(Duration::from_secs(60))
            .expect("owned grouped checkpoint absent");
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
    fn fixed_grouped_finish(&mut self, kill: bool, inspection: bool) -> u8 {
        if kill {
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
            "owned grouped child did not terminate"
        );
        let status = self.child.wait().unwrap();
        if kill {
            assert_eq!(status.signal(), Some(libc::SIGKILL));
        } else {
            assert!(status.success(), "owned grouped child failed: {status}");
        }
        let mut tail = Vec::new();
        self.output
            .as_mut()
            .unwrap()
            .read_to_end(&mut tail)
            .unwrap();
        assert!(
            tail.len() < BUDGET as usize && self.output.as_ref().unwrap().get_ref().limit() > 0
        );
        acknowledgements(&tail, self.child.id()).expect("ambiguous or foreign ACK")
    }
}
fn group_case(stage: &str, mode: &str, repeat: u32) {
    let s = Scratch::new();
    let mut child = OwnedChild::fixed_grouped_start(&s, mode, stage);
    assert!(!terminal(child.pidfd.as_ref().unwrap(), 0));
    assert!(matches!(
        lock_host_root(&s.campaigns()),
        Err(HostExecutionError::Busy)
    ));
    let origin = snapshot(&s.origin());
    let context = snapshot(&s.context());
    let authority = snapshot(s.1.path());
    // All grouped cuts occur after complete original Starting/pipes/sinks.
    checkpoint_contents(&s, "AfterReturn", &context, &authority);
    let kill = mode == "kill";
    let acks = child.fixed_grouped_finish(kill, false);
    assert_eq!(acks, if mode == "control" { 1 } else { 0 });
    drop(lock_host_root(&s.campaigns()).expect("original flock survived terminated grouped owner"));
    assert_eq!(snapshot(&s.origin()), origin);
    assert_eq!(snapshot(s.1.path()), authority);
    if kill {
        assert_eq!(snapshot(&s.context()), context);
    } else {
        assert_eq!(fs::read_dir(s.context()).unwrap().count(), 0);
    }
    let retained = snapshot(&s.context());
    for inspection in 1..=2 {
        let mut fresh = OwnedChild::fixed_grouped_start(&s, "inspect", stage);
        assert_eq!(fresh.fixed_grouped_finish(false, true), 0);
        assert_eq!(snapshot(&s.origin()), origin);
        assert_eq!(snapshot(&s.context()), retained);
        assert_eq!(snapshot(s.1.path()), authority);
        println!("HC074-FIXED-GROUPED-INSPECTION repeat={repeat} stage={stage} mode={mode} inspection={inspection} refused=true unchanged=true");
    }
    println!("HC074-FIXED-GROUPED-PROCESS seed=0x{GROUP_SEED:08x} repeat={repeat} stage={stage} mode={mode} sigkill={kill} acks={acks} held=true unchanged=true send=false");
}
#[test]
#[ignore = "manual nonroot grouped process screen; exact owned SIGKILL and ordinary ACK control"]
fn original_fixed_grouped_process_screen() {
    group_case("returned-unacknowledged", "control", 0);
    group_case("returned-unacknowledged", "kill", 0);
    println!(
        "HC074-FIXED-GROUPED-PROCESS-SCREEN sigkills=1 controls=1 inspections=4 completed=true"
    );
}
#[test]
#[ignore = "manual nonroot grouped preparation SIGKILL matrix and synthetic return ACK loss"]
fn original_fixed_grouped_process_matrix() {
    for repeat in 1..=3 {
        for stage in order() {
            group_case(stage, "kill", repeat);
        }
        group_case("returned-unacknowledged", "control", repeat);
        group_case("returned-unacknowledged", "ack-lost", repeat);
        println!("HC074-FIXED-GROUPED-PROCESS-SUMMARY seed=0x{GROUP_SEED:08x} repeat={repeat} sigkills=6 controls=1 lost_acks=1 inspections=16 admission=false");
    }
}
