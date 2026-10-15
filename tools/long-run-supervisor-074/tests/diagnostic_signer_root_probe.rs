#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
static COPY_AND_SPAWN: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Image {
    _temp: tempfile::TempDir,
    path: std::path::PathBuf,
    extra_libm: bool,
}
impl Image {
    fn new() -> Self {
        let _guard = COPY_AND_SPAWN.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("root-probe");
        fs::copy(env!("CARGO_BIN_EXE_diagnostic_signer_root_probe"), &path).unwrap();
        let out = Command::new("/usr/bin/strip")
            .arg("--strip-debug")
            .arg(&path)
            .env_clear()
            .output()
            .unwrap();
        assert!(out.status.success(), "private copy strip failed: {out:?}");
        assert!(
            fs::metadata(&path).unwrap().len() <= 8 * 1024 * 1024,
            "private stripped image still exceeds the unchanged bound"
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        // Independent declaration observation, not a runtime policy exception.
        // Local Rust1.88 adds libm and MUST refuse the unchanged four-role graph.
        let declarations = Command::new("/usr/bin/readelf")
            .arg("-d")
            .arg(&path)
            .env_clear()
            .output()
            .unwrap();
        assert!(declarations.status.success());
        let text = String::from_utf8(declarations.stdout).unwrap();
        let mut needed: Vec<_> = text
            .lines()
            .filter(|l| l.contains("(NEEDED)"))
            .map(|l| l.split_once('[').unwrap().1.strip_suffix(']').unwrap())
            .collect();
        needed.sort_unstable();
        let extra_libm = needed.contains(&"libm.so.6");
        if option_env!("RUSTUP_TOOLCHAIN").is_some_and(|v| v.starts_with("1.94.0")) {
            assert!(
                !extra_libm,
                "approved compiler must materialize, not pass by refusal"
            );
        }
        let expected = if extra_libm {
            vec![
                "ld-linux-x86-64.so.2",
                "libc.so.6",
                "libgcc_s.so.1",
                "libm.so.6",
            ]
        } else {
            vec!["ld-linux-x86-64.so.2", "libc.so.6", "libgcc_s.so.1"]
        };
        assert_eq!(needed, expected, "unattributed GNU profile drift");
        Self {
            _temp: temp,
            path,
            extra_libm,
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(&self.path);
        command
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
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
    let guard = COPY_AND_SPAWN.lock().unwrap();
    let mut child = command.spawn().unwrap();
    drop(guard);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("normal root preparation stalled");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn fixed_profile_outcome(image: &Image) {
    if image.extra_libm {
        refused(image.command().arg("prepare"));
        println!("extra_libm_profile=refused unchanged_four_role_graph=true admission=false");
        return;
    }
    let out = output(image.command().arg("prepare"));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        out.stdout,
        b"fixed root preparation probe passed; admission=false\n"
    );
    assert!(out.stderr.is_empty(), "{out:?}");
}
fn refused(command: &mut Command) {
    let out = output(command);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty());
    assert_eq!(out.stderr, b"Fixed root preparation refused.\n");
}
#[test]
fn normal_private_stripped_self_and_real_gnu_runtime_prepare_and_cleanup() {
    let image = Image::new();
    fixed_profile_outcome(&image);
    if !image.extra_libm {
        println!("normal_fixed_root_prepare=true real_runtime_sources=4 child_launch=false admission=false");
    }
}
#[test]
fn nonutf8_environment_writable_hardlinked_and_oversize_self_refuse() {
    let image = Image::new();
    refused(image.command().arg(std::ffi::OsString::from_vec(vec![255])));
    refused(
        image
            .command()
            .arg("prepare")
            .env("UNTRUSTED_ROOT_PROBE", "1"),
    );
    fs::set_permissions(&image.path, fs::Permissions::from_mode(0o755)).unwrap();
    refused(image.command().arg("prepare"));
    fs::set_permissions(&image.path, fs::Permissions::from_mode(0o555)).unwrap();
    let alias = image._temp.path().join("alias");
    fs::hard_link(&image.path, &alias).unwrap();
    refused(image.command().arg("prepare"));
    fs::remove_file(alias).unwrap();
    {
        let _guard = COPY_AND_SPAWN.lock().unwrap();
        fs::set_permissions(&image.path, fs::Permissions::from_mode(0o755)).unwrap();
        let writer = fs::OpenOptions::new()
            .write(true)
            .open(&image.path)
            .unwrap();
        writer.set_len(8 * 1024 * 1024 + 1).unwrap();
        drop(writer);
        fs::set_permissions(&image.path, fs::Permissions::from_mode(0o555)).unwrap();
    }
    refused(image.command().arg("prepare"));
}
#[test]
fn four_independent_normal_preparations_leave_parent_context_unchanged() {
    let before = (
        fs::read_link("/proc/self/root").unwrap(),
        std::env::current_dir().unwrap(),
        unsafe { (libc::getuid(), libc::getgid()) },
    );
    let jobs: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let image = Image::new();
                fixed_profile_outcome(&image);
            })
        })
        .collect();
    let results: Vec<_> = jobs.into_iter().map(|j| j.join()).collect();
    assert!(results.into_iter().all(|r| r.is_ok()));
    assert_eq!(
        before,
        (
            fs::read_link("/proc/self/root").unwrap(),
            std::env::current_dir().unwrap(),
            unsafe { (libc::getuid(), libc::getgid()) }
        )
    );
    println!(
        "independent_normal_root_preparations=4 parent_context_unchanged=true admission=false"
    );
}

#[test]
fn normal_original_root_exec_observes_selected_runtime_then_reaps() {
    let image = Image::new();
    if image.extra_libm {
        refused(image.command().arg("root-exec"));
        println!("extra_libm_root_exec=refused admission=false");
        return;
    }
    let out = output(image.command().arg("root-exec"));
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        out.stdout,
        b"selected rooted self-exec observed; admission=false\n"
    );
    assert!(out.stderr.is_empty(), "{out:?}");
    println!("normal_root_exec=true initial_images=2 loaded_images=4 original_reaped=true stream_eof=true admission=false");
}

#[test]
fn repeated_original_root_execs_leave_parent_context_unchanged() {
    let image = Image::new();
    let before = (
        fs::read_link("/proc/self/root").unwrap(),
        std::env::current_dir().unwrap(),
        fs::read_link("/proc/self/ns/user").unwrap(),
        unsafe { (libc::getuid(), libc::getgid()) },
    );
    for _ in 0..4 {
        if image.extra_libm {
            refused(image.command().arg("root-exec"));
        } else {
            let out = output(image.command().arg("root-exec"));
            assert_eq!(out.status.code(), Some(0), "{out:?}");
            assert_eq!(
                out.stdout,
                b"selected rooted self-exec observed; admission=false\n"
            );
            assert!(out.stderr.is_empty(), "{out:?}");
        }
    }
    assert_eq!(
        before,
        (
            fs::read_link("/proc/self/root").unwrap(),
            std::env::current_dir().unwrap(),
            fs::read_link("/proc/self/ns/user").unwrap(),
            unsafe { (libc::getuid(), libc::getgid()) }
        )
    );
    println!("independent_root_exec_owners=4 extra_libm_refusal={} parent_context_unchanged=true admission=false", image.extra_libm);
}
#[test]
fn root_exec_rejects_environment_extra_descriptors_and_ignored_sigchld() {
    let image = Image::new();
    refused(
        image
            .command()
            .arg("root-exec")
            .env("UNTRUSTED_ROOT_PROBE", "1"),
    );
    refused(image.command().args(["root-exec", "extra"]));
    let out = output(image.command().arg("--root-child"));
    assert_eq!(out.status.code(), Some(92));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
    let mut inherited = image.command();
    unsafe {
        inherited.pre_exec(|| {
            let fd = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::dup2(fd, 200) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            libc::close(fd);
            Ok(())
        });
    }
    refused(inherited.arg("root-exec"));
    let mut ignored = image.command();
    unsafe {
        ignored.pre_exec(|| {
            if libc::signal(libc::SIGCHLD, libc::SIG_IGN) == libc::SIG_ERR {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    refused(ignored.arg("root-exec"));
}
