#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

// Only copy/exec startup is serialized. Original normal owners run concurrently
// after spawn. A pre_exec CLOEXEC sweep alone cannot prevent another fork child
// from retaining a temporary writer before it reaches exec.
static COPY_AND_SPAWN: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Image {
    temp: tempfile::TempDir,
    path: std::path::PathBuf,
}
impl Image {
    fn new() -> Self {
        let _copy = COPY_AND_SPAWN.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("owned-probe");
        fs::copy(env!("CARGO_BIN_EXE_diagnostic_signer_owned_probe"), &path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        Self { temp, path }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(&self.path);
        command
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // SAFETY: only the fresh test child table's flags; no parent descriptors change.
        unsafe {
            command.pre_exec(|| {
                if libc::syscall(
                    libc::SYS_close_range,
                    3u32,
                    u32::MAX,
                    libc::CLOSE_RANGE_CLOEXEC,
                ) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command
    }
}
fn output(command: &mut Command) -> Output {
    let launch = COPY_AND_SPAWN.lock().unwrap();
    let mut child = command.spawn().unwrap();
    drop(launch);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("owned original probe stalled");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn passed(image: &Image, case: &str) {
    let result = output(image.command().arg(case));
    assert_eq!(result.status.code(), Some(0), "{case}: {result:?}");
    assert_eq!(
        result.stdout,
        b"owned original self-exec probe passed; admission=false\n"
    );
    assert!(result.stderr.is_empty());
}
fn refused(command: &mut Command) {
    let result = output(command);
    assert_eq!(result.status.code(), Some(2), "{result:?}");
    assert!(result.stdout.is_empty());
    assert_eq!(result.stderr, b"Fixed owned probe refused.\n");
}

#[test]
fn ordinary_owned_probe_native_and_two_abi_modes_use_original_exec() {
    let image = Image::new();
    for case in ["native", "x32", "i386"] {
        passed(&image, case);
    }
    println!("normal_original_exec_modes=native,x32,i386 admission=false");
}
#[test]
fn ordinary_owned_probe_arguments_internal_child_and_environment_refuse() {
    let image = Image::new();
    refused(&mut image.command());
    for case in [
        "--exec",
        "--pid",
        "--child-native",
        "--child-x32",
        "--child-i386",
    ] {
        refused(image.command().arg(case));
    }
    refused(image.command().arg("native").arg("x32"));
    let mut ignored = image.command();
    ignored.arg("native");
    // SAFETY: only this fresh test child's disposition, never the test parent's.
    // SIG_IGN survives exec and must refuse before the normal owner can fork.
    unsafe {
        ignored.pre_exec(|| {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = libc::SIG_IGN;
            if libc::sigaction(libc::SIGCHLD, &action, std::ptr::null_mut()) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    refused(&mut ignored);
    refused(
        image
            .command()
            .arg(std::ffi::OsString::from_vec(vec![0xff])),
    );
    refused(
        image
            .command()
            .arg("native")
            .env("HYDRACACHE_UNTRUSTED_OWNED_PROBE", "1"),
    );
}
#[test]
fn ordinary_owned_probe_writable_and_hardlinked_image_refuse() {
    let image = Image::new();
    fs::set_permissions(&image.path, fs::Permissions::from_mode(0o755)).unwrap();
    refused(image.command().arg("native"));
    fs::set_permissions(&image.path, fs::Permissions::from_mode(0o555)).unwrap();
    fs::hard_link(&image.path, image.temp.path().join("alias")).unwrap();
    refused(image.command().arg("native"));
}

#[test]
fn ordinary_owned_probe_kernel_busy_writer_control_is_not_retried() {
    let image = Image::new();
    let launch = COPY_AND_SPAWN.lock().unwrap();
    fs::set_permissions(&image.path, fs::Permissions::from_mode(0o755)).unwrap();
    let writer = fs::OpenOptions::new()
        .write(true)
        .open(&image.path)
        .unwrap();
    fs::set_permissions(&image.path, fs::Permissions::from_mode(0o555)).unwrap();
    let error = match image.command().arg("native").spawn() {
        Err(error) => error,
        Ok(mut unexpected) => {
            let _ = unexpected.kill();
            let _ = unexpected.wait();
            panic!("live original writer must prevent exec");
        }
    };
    assert_eq!(error.raw_os_error(), Some(libc::ETXTBSY));
    drop(writer);
    drop(launch);
    passed(&image, "native");
    println!(
        "held_writer_exec=ETXTBSY retry=false after_explicit_writer_drop=new_independent_probe"
    );
}
fn context() -> (String, (libc::rlim_t, libc::rlim_t), std::path::PathBuf) {
    let raw = fs::read_to_string("/proc/thread-self/status").unwrap();
    let status = raw
        .lines()
        .filter(|line| {
            ["NoNewPrivs:", "Seccomp:", "Seccomp_filters:"]
                .iter()
                .any(|key| line.starts_with(key))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut limits = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limits) },
        0
    );
    (
        status,
        (limits.rlim_cur, limits.rlim_max),
        std::env::current_dir().unwrap(),
    )
}
#[test]
fn ordinary_owned_probe_four_independent_normal_owners_preserve_parent() {
    let before = context();
    let owners: Vec<_> = (0..4)
        .map(|index| {
            std::thread::spawn(move || {
                let image = Image::new();
                let mut cases = ["native", "i386", "x32"];
                cases.rotate_left((0x0740c07ausize + index) % 3);
                for case in cases {
                    passed(&image, case);
                }
            })
        })
        .collect();
    for owner in owners {
        owner.join().unwrap();
    }
    assert_eq!(before, context());
    println!("normal_independent_owners=4 original_children=12 parent_context_unchanged=true seed=0x0740c07a");
}
