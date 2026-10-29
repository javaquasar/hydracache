//! One isolated output-buffer candidate, measured with complete original calls.
use super::*;
use std::hint::black_box;

const CASES: [&str; 7] = ["proc-fs", "ns-fs", "boot", "user", "mnt", "pid", "time"];
const ARMS: [&str; 4] = ["reference", "reference-control", "runtime", "prototype"];
const ORDERS: [[usize; 4]; 8] = [
    [0, 1, 2, 3],
    [1, 2, 3, 0],
    [2, 3, 0, 1],
    [3, 0, 1, 2],
    [0, 3, 2, 1],
    [1, 0, 3, 2],
    [2, 1, 0, 3],
    [3, 2, 1, 0],
];

fn reference_filesystem(file: &File, magic: libc::c_long) -> Result<(), KernelError> {
    #[cfg(test)]
    let _operation_fs = operation_cost::span(operation_cost::Kind::Filesystem);
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type != magic {
        return Err(kernel_invalid!("filesystem-type"));
    }
    Ok(())
}
fn reference_fixed_child(
    parent: &File,
    name: &std::ffi::CStr,
    directory: bool,
) -> Result<File, KernelError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    #[cfg(test)]
    let operation_open = operation_cost::span(operation_cost::Kind::Open);
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    #[cfg(test)]
    drop(operation_open);
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    reference_filesystem(&file, 0x9fa0)?;
    Ok(file)
}
fn prototype_fixed_child(
    parent: &File,
    name: &std::ffi::CStr,
    directory: bool,
) -> Result<File, KernelError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    #[cfg(test)]
    let operation_open = operation_cost::span(operation_cost::Kind::Open);
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    #[cfg(test)]
    drop(operation_open);
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    filesystem_projection::check(&file, 0x9fa0)?;
    Ok(file)
}
fn reference_boot_apply<T>(
    root: &File,
    decode: impl FnOnce(([u8; 38], usize)) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    #[cfg(test)]
    drop(operation_cost::span(operation_cost::Kind::Empty));
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = reference_fixed_child(root, c"sys", true)?;
    let kernel = reference_fixed_child(&sys, c"kernel", true)?;
    let random = reference_fixed_child(&kernel, c"random", true)?;
    let file = reference_fixed_child(&random, c"boot_id", false)?;
    let raw = boot_read(|bytes| file.read_at(bytes, 0))?;
    drop(file);
    decode(raw)
}
fn prototype_boot_apply<T>(
    root: &File,
    decode: impl FnOnce(([u8; 38], usize)) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    #[cfg(test)]
    drop(operation_cost::span(operation_cost::Kind::Empty));
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = prototype_fixed_child(root, c"sys", true)?;
    let kernel = prototype_fixed_child(&sys, c"kernel", true)?;
    let random = prototype_fixed_child(&kernel, c"random", true)?;
    let file = prototype_fixed_child(&random, c"boot_id", false)?;
    let raw = boot_read(|bytes| file.read_at(bytes, 0))?;
    drop(file);
    decode(raw)
}
fn reference_namespace(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    drop(operation_cost::span(operation_cost::Kind::Empty));
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    #[cfg(test)]
    let operation_open = operation_cost::span(operation_cost::Kind::Open);
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(namespace_path(name).as_ref())?;
    #[cfg(test)]
    drop(operation_open);
    reference_filesystem(&file, 0x6e736673)?;
    #[cfg(test)]
    let operation_identity = operation_cost::span(operation_cost::Kind::Identity);
    let id = fd_identity::fresh(&file)?;
    #[cfg(test)]
    drop(operation_identity);
    Ok((file, id))
}
fn prototype_namespace(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    drop(operation_cost::span(operation_cost::Kind::Empty));
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    #[cfg(test)]
    let operation_open = operation_cost::span(operation_cost::Kind::Open);
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(namespace_path(name).as_ref())?;
    #[cfg(test)]
    drop(operation_open);
    filesystem_projection::check(&file, 0x6e736673)?;
    #[cfg(test)]
    let operation_identity = operation_cost::span(operation_cost::Kind::Identity);
    let id = fd_identity::fresh(&file)?;
    #[cfg(test)]
    drop(operation_identity);
    Ok((file, id))
}
fn class<T>(value: Result<T, KernelError>) -> (&'static str, Option<i32>) {
    match value {
        Ok(_) => ("ok", None),
        Err(KernelError::Io(e)) => ("io", e.raw_os_error()),
        Err(KernelError::Invalid) => ("invalid", None),
        Err(e) => panic!("unexpected {e:?}"),
    }
}
#[test]
fn filesystem_projection_real_objects_errors_and_fd_state_match() {
    let temp = tempfile::NamedTempFile::new().unwrap();
    let proc = File::open("/proc").unwrap();
    let ns = File::open("/proc/thread-self/ns/user").unwrap();
    let null = File::open("/dev/null").unwrap();
    let (socket, _) = std::os::unix::net::UnixStream::pair().unwrap();
    let socket = File::from(OwnedFd::from(socket));
    for file in [temp.as_file(), &proc, &ns, &null, &socket] {
        let id = fd_identity::fresh(file).unwrap();
        let before: Vec<_> = [libc::F_GETFL, libc::F_GETFD]
            .map(|op| unsafe { libc::fcntl(file.as_raw_fd(), op) })
            .into();
        for magic in [0x9fa0, 0x6e736673, 0, -1, libc::c_long::MAX] {
            assert_eq!(
                class(reference_filesystem(file, magic)),
                class(filesystem_projection::check(file, magic))
            );
        }
        assert_eq!(fd_identity::fresh(file).unwrap(), id);
        assert_eq!(
            before,
            [libc::F_GETFL, libc::F_GETFD].map(|op| unsafe { libc::fcntl(file.as_raw_fd(), op) })
        );
    }
    // A failed call must not inspect its uninitialized output or retry EINTR.
    let failure =
        unsafe { filesystem_projection::query_once(-1, 0x9fa0, |fd, out| libc::fstatfs(fd, out)) };
    assert_eq!(class(failure), ("io", Some(libc::EBADF)));
}
#[test]
fn filesystem_projection_partial_output_and_single_query_faults() {
    let mut seed = 0x074f5a17_u64;
    let mut cases = 0;
    for _ in 0..1024 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let actual = seed as libc::c_long;
        for expected in [actual, actual ^ 1] {
            let mut calls = 0;
            let observed = unsafe {
                filesystem_projection::query_once(74, expected, |fd, out| {
                    assert_eq!(fd, 74);
                    calls += 1;
                    std::ptr::addr_of_mut!((*out).f_type).write(actual);
                    0
                })
            };
            assert_eq!(
                class(observed),
                if actual == expected {
                    ("ok", None)
                } else {
                    ("invalid", None)
                }
            );
            assert_eq!(calls, 1);
            cases += 1;
        }
    }
    for code in [
        libc::EINTR,
        libc::EBADF,
        libc::EIO,
        libc::EACCES,
        libc::EOVERFLOW,
    ] {
        for status in [-1, 1] {
            let mut calls = 0;
            let observed = unsafe {
                filesystem_projection::query_once(-74, 0x9fa0, |fd, _out| {
                    assert_eq!(fd, -74);
                    calls += 1;
                    *libc::__errno_location() = code;
                    status
                })
            };
            assert_eq!(class(observed), ("io", Some(code)));
            assert_eq!(calls, 1);
            cases += 1;
        }
    }
    println!("HC074-FILESYSTEM-FAULT seed=0x074f5a17 cases={cases} single_query=true partial_success_field_only=true");
}
fn invoke(
    case: usize,
    variant: usize,
    root: &File,
    ns: &File,
    boot: &str,
    ns_id: (u64, u64),
) -> Result<(), KernelError> {
    match case {
        0 | 1 => {
            let (file, magic) = if case == 0 {
                (root, 0x9fa0)
            } else {
                (ns, 0x6e736673)
            };
            match variant {
                0 | 1 => reference_filesystem(file, magic),
                2 => filesystem(file, magic),
                3 => filesystem_projection::check(file, magic),
                _ => unreachable!(),
            }
        }
        2 => {
            let decode = |raw| boot_comparison::view(&raw).map(|value| value == boot);
            let matched = match variant {
                0 | 1 => reference_boot_apply(root, decode),
                2 => boot_comparison::matches(root, boot),
                3 => prototype_boot_apply(root, decode),
                _ => unreachable!(),
            }?;
            assert!(matched);
            Ok(())
        }
        _ => {
            let value = match variant {
                0 | 1 => reference_namespace(CASES[case]),
                2 => namespace(CASES[case]),
                3 => prototype_namespace(CASES[case]),
                _ => unreachable!(),
            }?;
            assert_eq!(value.1, ns_id);
            drop(value);
            Ok(())
        }
    }
}
#[test]
fn filesystem_projection_full_paths_preserve_values_counts_and_refusals() {
    let origin = Origin::open().unwrap();
    for (case, name) in CASES.iter().enumerate() {
        let ns = File::open(
            if case < 3 {
                namespace_path("user")
            } else {
                namespace_path(name)
            }
            .as_ref(),
        )
        .unwrap();
        let ns_id = fd_identity::fresh(&ns).unwrap();
        for (index, kind) in operation_cost::KINDS.into_iter().enumerate() {
            let capture = operation_cost::Capture::new(kind).unwrap();
            invoke(case, 3, &origin.root, &ns, &origin.boot, ns_id).unwrap();
            let report = capture.finish();
            assert!(!report.clock_failed);
            let expected = if case < 2 {
                [0, 1, 0, 0, 0, 0, 0]
            } else if case == 2 {
                [4, 4, 0, 1, 1, 0, 1]
            } else {
                [1, 1, 1, 0, 0, 1, 1]
            };
            assert_eq!(report.calls, expected[index]);
        }
    }
    for name in ["absent-hc074", "bad\0name"] {
        assert_eq!(
            class(reference_namespace(name)),
            class(prototype_namespace(name))
        );
    }
    let wrong = File::open("/dev/null").unwrap();
    for variant in [0, 2, 3] {
        assert_eq!(
            class(invoke(2, variant, &wrong, &wrong, &origin.boot, (0, 0))),
            ("io", Some(libc::ENOTDIR))
        );
    }
    // Wrong filesystem refuses before decode or namespace identity.
    for kind in operation_cost::KINDS {
        let capture = operation_cost::Capture::new(kind).unwrap();
        assert_eq!(
            class(filesystem_projection::check(&origin.root, 0x6e736673)),
            ("invalid", None)
        );
        assert_eq!(
            capture.finish().calls,
            u64::from(kind == operation_cost::Kind::Filesystem)
        );
    }
}
fn screen(installed: bool) {
    let cohort: usize = std::env::var("HC074_FILESYSTEM_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    assert_eq!(unsafe { libc::geteuid() }, 1000);
    let phase = if installed { "installed" } else { "prototype" };
    let origin = Origin::open().unwrap();
    println!(
        "HC074-FILESYSTEM-ORIGIN {}",
        serde_json::json!({"cohort":cohort,"phase":phase,"uid":1000,"samples":16,"iterations":1024,"warmups":32,"product":false,"admission":false})
    );
    for (case, name) in CASES.iter().enumerate() {
        let ns = File::open(
            if case < 3 {
                namespace_path("user")
            } else {
                namespace_path(name)
            }
            .as_ref(),
        )
        .unwrap();
        let ns_id = fd_identity::fresh(&ns).unwrap();
        for variant in 0..4 {
            for _ in 0..32 {
                invoke(case, variant, &origin.root, &ns, &origin.boot, ns_id).unwrap();
            }
        }
        for sample in 0..16 {
            for (slot, variant) in ORDERS[(sample + cohort) % 8].into_iter().enumerate() {
                let variant = black_box(variant);
                let before = operation_cost::cpu().unwrap();
                let start = std::time::Instant::now();
                for _ in 0..1024 {
                    black_box(invoke(
                        black_box(case),
                        variant,
                        &origin.root,
                        &ns,
                        &origin.boot,
                        ns_id,
                    ))
                    .unwrap();
                }
                let wall_ns = start.elapsed().as_nanos();
                let cpu_ns = operation_cost::cpu().unwrap().checked_sub(before).unwrap();
                println!(
                    "HC074-FILESYSTEM-COST {}",
                    serde_json::json!({"cohort":cohort,"phase":phase,"case":name,"sample":sample,"slot":slot,"variant":ARMS[variant],"calls":1024,"opens_per_call":if case<2 {0} else if case==2 {4} else {1},"filesystem_checks":if case==2 {4} else {1},"identity_checks":if case>2 {1} else {0},"cpu_ns":cpu_ns,"wall_ns":wall_ns,"product":false,"admission":false})
                );
            }
        }
    }
}
#[test]
#[ignore = "manual finite baseline filesystem projection screen; no preparation retry or admission"]
fn filesystem_projection_prototype_cpu_screen() {
    screen(false);
}
#[test]
#[ignore = "manual finite integrated filesystem projection screen; no preparation retry or admission"]
fn filesystem_projection_installed_cpu_screen() {
    screen(true);
}
