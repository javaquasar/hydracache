//! Whole-Namespace comparison of one fixed open wrapper; no active operation spans.
use super::*;
use std::hint::black_box;
use std::io::{BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

// FROZEN WHOLE-METHOD ARMS

fn reference(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    drop(operation_cost::span(operation_cost::Kind::Empty));
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    // These fixed kernel magic links must be followed, then checked as nsfs.
    #[cfg(test)]
    let operation_open = operation_cost::span(operation_cost::Kind::Open);
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(namespace_path(name).as_ref())?;
    #[cfg(test)]
    drop(operation_open);
    filesystem(&file, 0x6e736673)?;
    #[cfg(test)]
    let operation_identity = operation_cost::span(operation_cost::Kind::Identity);
    let id = fd_identity::fresh(&file)?;
    #[cfg(test)]
    drop(operation_identity);
    Ok((file, id))
}

fn prototype(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    drop(operation_cost::span(operation_cost::Kind::Empty));
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    // These fixed kernel magic links must be followed, then checked as nsfs.
    #[cfg(test)]
    let operation_open = operation_cost::span(operation_cost::Kind::Open);
    let file = fixed_namespace_open::read(name)?;
    #[cfg(test)]
    drop(operation_open);
    filesystem(&file, 0x6e736673)?;
    #[cfg(test)]
    let operation_identity = operation_cost::span(operation_cost::Kind::Identity);
    let id = fd_identity::fresh(&file)?;
    #[cfg(test)]
    drop(operation_identity);
    Ok((file, id))
}

fn class<T>(value: Result<T, std::io::Error>) -> (bool, std::io::ErrorKind, Option<i32>) {
    match value {
        Ok(_) => (true, std::io::ErrorKind::Other, None),
        Err(e) => (false, e.kind(), e.raw_os_error()),
    }
}
fn standard(path: &std::ffi::CStr) -> std::io::Result<File> {
    use std::os::unix::ffi::OsStrExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(std::ffi::OsStr::from_bytes(path.to_bytes()))
}
#[test]
fn namespace_open_flags_identity_errors_and_fallback_match() {
    let temp = tempfile::tempdir().unwrap();
    let regular = temp.path().join("file");
    std::fs::write(&regular, b"x").unwrap();
    let forbidden = temp.path().join("forbidden");
    std::fs::write(&forbidden, b"x").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&forbidden, std::fs::Permissions::from_mode(0o0)).unwrap();
    let alias = temp.path().join("alias");
    std::os::unix::fs::symlink(&regular, &alias).unwrap();
    for path in [
        regular.clone(),
        forbidden,
        temp.path().join("absent"),
        regular.join("child"),
        alias,
        std::path::PathBuf::from("/proc/thread-self/ns/user"),
    ] {
        use std::os::unix::ffi::OsStrExt;
        let path = CString::new(path.as_os_str().as_bytes()).unwrap();
        let a = standard(&path);
        let b = fixed_namespace_open::open_literal(&path);
        if let (Ok(a), Ok(b)) = (&a, &b) {
            assert_eq!(
                fd_identity::fresh(a).unwrap(),
                fd_identity::fresh(b).unwrap()
            );
            for command in [libc::F_GETFL, libc::F_GETFD] {
                assert_eq!(unsafe { libc::fcntl(a.as_raw_fd(), command) }, unsafe {
                    libc::fcntl(b.as_raw_fd(), command)
                });
            }
            assert_ne!(
                unsafe { libc::fcntl(b.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        assert_eq!(class(a), class(b));
    }
    for name in [
        "user",
        "mnt",
        "pid",
        "time",
        "net",
        "absent-hc074",
        "bad\0name",
        "",
    ] {
        let a = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(namespace_path(name).as_ref());
        let b = fixed_namespace_open::read(name);
        if let (Ok(a), Ok(b)) = (&a, &b) {
            assert_eq!(
                fd_identity::fresh(a).unwrap(),
                fd_identity::fresh(b).unwrap()
            );
        }
        assert_eq!(class(a), class(b), "{name:?}");
    }
}
#[test]
fn namespace_open_retry_contract_keeps_terminal_error_and_first_success() {
    for interruptions in [0, 1, 2, 17, 128] {
        for terminal in [
            None,
            Some(libc::EIO),
            Some(libc::EACCES),
            Some(libc::EMFILE),
        ] {
            let mut calls = 0;
            let observed = fixed_namespace_open::retry_interrupted(|| {
                calls += 1;
                if calls <= interruptions {
                    return Err(std::io::Error::from_raw_os_error(libc::EINTR));
                }
                match terminal {
                    Some(code) => Err(std::io::Error::from_raw_os_error(code)),
                    None => Ok(74),
                }
            });
            assert_eq!(calls, interruptions + 1);
            match terminal {
                Some(code) => assert_eq!(observed.unwrap_err().raw_os_error(), Some(code)),
                None => assert_eq!(observed.unwrap(), 74),
            }
        }
    }
}
#[test]
fn namespace_open_full_methods_keep_live_observation_counts_and_identity() {
    for name in NAMES {
        let old = reference(name).unwrap();
        let proposed = prototype(name).unwrap();
        let actual = namespace(name).unwrap();
        assert_eq!(old.1, proposed.1);
        assert_eq!(old.1, actual.1);
        for (index, kind) in operation_cost::KINDS.into_iter().enumerate() {
            let scope = operation_cost::Capture::new(kind).unwrap();
            drop(prototype(name).unwrap());
            let report = scope.finish();
            assert!(!report.clock_failed);
            assert_eq!(report.calls, [1, 1, 1, 0, 0, 1, 1][index]);
        }
    }
}
static SIGNALS: AtomicU32 = AtomicU32::new(0);
extern "C" fn interrupted(_: libc::c_int) {
    // Only the dedicated fixture child installs this handler; preserve errno.
    let errno = unsafe { libc::__errno_location() };
    let saved = unsafe { *errno };
    SIGNALS.fetch_add(1, Ordering::Relaxed);
    let message = b"HC074-OPEN-INT\n";
    unsafe {
        libc::write(libc::STDOUT_FILENO, message.as_ptr().cast(), message.len());
        *errno = saved;
    }
}
struct ChildGuard(Option<Child>);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn wait_blocked(child: &mut Child, tid: i32) -> (i64, u64) {
    let stop = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(
            child.try_wait().unwrap().is_none(),
            "fixture exited before blocked open"
        );
        let text =
            std::fs::read_to_string(format!("/proc/{}/task/{tid}/syscall", child.id())).unwrap();
        let fields: Vec<_> = text.split_whitespace().collect();
        if let Some(number) = fields.first().and_then(|s| s.parse::<i64>().ok()) {
            let position = if number == libc::SYS_openat {
                Some(3)
            } else if number == libc::SYS_open {
                Some(2)
            } else {
                None
            };
            if let Some(position) = position {
                return (
                    number,
                    u64::from_str_radix(fields[position].trim_start_matches("0x"), 16).unwrap(),
                );
            }
        }
        assert!(Instant::now() < stop, "fixture did not block in open");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn fifo_control(role: &str) -> (i64, u64, i64, i64) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("fifo");
    use std::os::unix::ffi::OsStrExt;
    let cpath = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
    let test = std::thread::current().name().unwrap().to_owned();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([&test, "--exact", "--nocapture", "--test-threads=1"])
        .env("HC074_OPEN_FIFO_CHILD", role)
        .env("HC074_OPEN_FIFO_PATH", &path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut guard = ChildGuard(Some(child));
    let stdout = guard.0.as_mut().unwrap().stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let line = line.unwrap();
            if let Some(start) = line.find("HC074-OPEN-") {
                let _ = tx.send(line[start..].to_owned());
            }
        }
    });
    let ready = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let tid: i32 = ready
        .strip_prefix("HC074-OPEN-READY ")
        .unwrap()
        .parse()
        .unwrap();
    let entry = wait_blocked(guard.0.as_mut().unwrap(), tid);
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_tgkill,
                guard.0.as_ref().unwrap().id() as i32,
                tid,
                libc::SIGUSR1,
            )
        },
        0
    );
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        "HC074-OPEN-INT"
    );
    assert_eq!(wait_blocked(guard.0.as_mut().unwrap(), tid), entry);
    let stop = Instant::now() + Duration::from_secs(5);
    let writer = loop {
        match OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&path)
        {
            Ok(file) => break file,
            Err(e) if e.raw_os_error() == Some(libc::ENXIO) => {
                assert!(Instant::now() < stop);
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(e) => panic!("writer: {e}"),
        }
    };
    let done = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(done.strip_prefix("HC074-OPEN-DONE ").unwrap()).unwrap();
    assert_eq!(value["signals"], 1);
    drop(writer);
    let stop = Instant::now() + Duration::from_secs(5);
    while guard.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(Instant::now() < stop);
        std::thread::sleep(Duration::from_millis(1));
    }
    let output = guard.0.take().unwrap().wait_with_output().unwrap();
    reader.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        entry.0,
        entry.1,
        value["fd_flags"].as_i64().unwrap(),
        value["file_flags"].as_i64().unwrap(),
    )
}
#[test]
fn namespace_open_real_fifo_eintr_preserves_std_behavior() {
    if let Ok(role) = std::env::var("HC074_OPEN_FIFO_CHILD") {
        assert!(matches!(role.as_str(), "reference" | "prototype"));
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        action.sa_sigaction = interrupted as *const () as usize;
        assert_eq!(unsafe { libc::sigemptyset(&mut action.sa_mask) }, 0);
        assert_eq!(
            unsafe { libc::sigaction(libc::SIGUSR1, &action, std::ptr::null_mut()) },
            0
        );
        println!("HC074-OPEN-READY {}", unsafe {
            libc::syscall(libc::SYS_gettid)
        });
        std::io::stdout().flush().unwrap();
        use std::os::unix::ffi::OsStrExt;
        let raw = std::env::var_os("HC074_OPEN_FIFO_PATH").unwrap();
        let path = CString::new(raw.as_bytes()).unwrap();
        let file = if role == "reference" {
            standard(&path)
        } else {
            fixed_namespace_open::open_literal(&path)
        }
        .unwrap();
        println!(
            "HC074-OPEN-DONE {}",
            serde_json::json!({"signals":SIGNALS.load(Ordering::Relaxed),"fd_flags":unsafe{libc::fcntl(file.as_raw_fd(),libc::F_GETFD)},"file_flags":unsafe{libc::fcntl(file.as_raw_fd(),libc::F_GETFL)}})
        );
        return;
    }
    let reference = fifo_control("reference");
    let prototype = fifo_control("prototype");
    assert_eq!(reference, prototype);
    println!(
        "HC074-OPEN-FIFO {}",
        serde_json::json!({"reference":reference,"prototype":prototype,"targeted_sigusr1":true,"product":false,"admission":false})
    );
}
fn screen(installed: bool) {
    let cohort: usize = std::env::var("HC074_NAMESPACE_OPEN_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    assert_eq!(unsafe { libc::geteuid() }, 1000);
    let phase = if installed { "installed" } else { "prototype" };
    type Call = fn(&str) -> Result<(File, (u64, u64)), KernelError>;
    let calls: [Call; 4] = [reference, reference, namespace, prototype];
    let labels = ["reference", "reference-control", "runtime", "prototype"];
    let orders = [
        [0, 1, 2, 3],
        [1, 2, 3, 0],
        [2, 3, 0, 1],
        [3, 0, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 3, 2],
        [2, 1, 0, 3],
        [3, 2, 1, 0],
    ];
    println!(
        "HC074-NAMESPACE-OPEN-ORIGIN {}",
        serde_json::json!({"cohort":cohort,"phase":phase,"uid":1000,"samples":16,"iterations":1024,"warmups":32,"product":false,"admission":false})
    );
    for name in NAMES {
        let expected = namespace(name).unwrap().1;
        for call in calls {
            for _ in 0..32 {
                let result = call(name).unwrap();
                assert_eq!(result.1, expected);
                drop(result);
            }
        }
        for sample in 0..16 {
            for (slot, variant) in orders[(sample + cohort) % 8].into_iter().enumerate() {
                let call = black_box(calls[variant]);
                let before = operation_cost::cpu().unwrap();
                let start = Instant::now();
                for _ in 0..1024 {
                    let result = black_box(call(black_box(name))).unwrap();
                    assert_eq!(result.1, expected);
                    black_box(result.1);
                    drop(result);
                }
                let wall_ns = start.elapsed().as_nanos();
                let cpu_ns = operation_cost::cpu().unwrap().checked_sub(before).unwrap();
                println!(
                    "HC074-NAMESPACE-OPEN-COST {}",
                    serde_json::json!({"cohort":cohort,"phase":phase,"case":name,"sample":sample,"slot":slot,"variant":labels[variant],"calls":1024,"opens_per_call":1,"filesystem_checks":1,"identity_checks":1,"cpu_ns":cpu_ns,"wall_ns":wall_ns,"product":false,"admission":false})
                );
            }
        }
    }
}
#[test]
#[ignore = "manual finite baseline Namespace open CPU screen; no preparation retry or admission"]
fn namespace_open_prototype_cpu_screen() {
    screen(false);
}
#[test]
#[ignore = "manual finite integrated Namespace open CPU screen; no preparation retry or admission"]
fn namespace_open_installed_cpu_screen() {
    screen(true);
}
