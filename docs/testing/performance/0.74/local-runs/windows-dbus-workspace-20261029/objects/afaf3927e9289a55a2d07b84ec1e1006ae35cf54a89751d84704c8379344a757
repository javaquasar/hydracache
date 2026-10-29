use super::super::*;
use super::{fresh, from_fd};
use std::fs;
use std::hint::black_box;
use std::io::{self, Write};

fn reference(file: &File) -> io::Result<(u64, u64)> {
    let m = file.metadata()?;
    Ok((m.dev(), m.ino()))
}

fn reference_namespace_current(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    let _span = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(namespace_path(name).as_ref())?;
    filesystem(&file, 0x6e736673)?;
    let id = reference(&file)?;
    Ok((file, id))
}

fn reference_origin(origin: &Origin) -> Result<(), KernelError> {
    let _span = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Origin,
    );
    let named = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/proc")?;
    for file in [&named, &origin.root] {
        filesystem(file, 0x9fa0)?;
        if reference(file)? != origin.id || !boot_comparison::matches(file, &origin.boot)? {
            return Err(kernel_invalid!("proc-origin-or-boot"));
        }
    }
    for (index, (file, id)) in origin.namespaces.iter().enumerate() {
        filesystem(file, 0x6e736673)?;
        if reference(file)? != *id || reference_namespace_current(NAMES[index])?.1 != *id {
            return Err(kernel_invalid!("namespace-origin"));
        }
    }
    Ok(())
}

#[test]
fn fresh_fd_identity_real_objects_flags_and_replacement_are_differential() {
    for name in [
        "/proc",
        "/proc/thread-self/ns/user",
        "/proc/thread-self/ns/mnt",
        "/proc/thread-self/ns/pid",
        "/proc/thread-self/ns/time",
        "/dev/null",
        "/sys",
    ] {
        let file = File::open(name).unwrap();
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        for _ in 0..32 {
            assert_eq!(fresh(&file).unwrap(), reference(&file).unwrap());
        }
        assert_eq!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) },
            flags
        );
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("held");
    fs::write(&path, b"original").unwrap();
    let held = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let original = fresh(&held).unwrap();
    fs::remove_file(&path).unwrap();
    fs::write(&path, b"replacement").unwrap();
    assert_eq!(fresh(&held).unwrap(), original);
    assert_eq!(fresh(&held).unwrap(), reference(&held).unwrap());
    assert_ne!(fresh(&File::open(&path).unwrap()).unwrap(), original);
    let mut writer = &held;
    writer.write_all(b"changed").unwrap();
    assert_eq!(fresh(&held).unwrap(), reference(&held).unwrap());
}

#[test]
fn fresh_fd_identity_bad_fd_errors_do_not_fabricate_identity() {
    for fd in [-1, i32::MAX] {
        let error = from_fd(fd).unwrap_err();
        assert_eq!(
            error.kind(),
            io::Error::from_raw_os_error(libc::EBADF).kind()
        );
        assert_eq!(error.raw_os_error(), Some(libc::EBADF));
    }
}

#[test]
fn fresh_fd_identity_original_trace_order_counts_and_refusal_sites_match() {
    use crate::diagnostic_ipc::validation_trace::kernel_walk::{capture, Kind};
    use crate::diagnostic_kernel_admission::refusal_trace;
    for case in 0..8 {
        let mut origin = Origin::open().unwrap();
        match case {
            0 => {}
            1 => origin.id.1 += 1,
            2 => origin.boot = "wrong".into(),
            3..=6 => origin.namespaces[case - 3].1 .1 += 1,
            _ => origin.namespaces[0].0 = tempfile::tempfile().unwrap(),
        }
        let ((old, old_site), old_trace) =
            capture(|| refusal_trace::capture(|| reference_origin(&origin)));
        let ((new, new_site), new_trace) =
            capture(|| refusal_trace::capture(|| origin.revalidate()));
        assert_eq!(format!("{old:?}"), format!("{new:?}"));
        assert_eq!(old_site.map(|s| s.site), new_site.map(|s| s.site));
        assert_eq!(
            old_trace.nodes.map(|n| n.calls),
            new_trace.nodes.map(|n| n.calls)
        );
        if case == 0 {
            assert_eq!(new_trace.nodes[Kind::Origin as usize].calls, 1);
            assert_eq!(new_trace.nodes[Kind::Boot as usize].calls, 2);
            assert_eq!(new_trace.nodes[Kind::Namespace as usize].calls, 4);
        } else {
            assert!(origin.revalidate().is_err());
            assert!(origin.sample().is_err());
        }
    }
}

fn cpu() -> u64 {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut value) },
        0
    );
    assert!(value.tv_sec >= 0 && (0..1_000_000_000).contains(&value.tv_nsec));
    (value.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(value.tv_nsec as u64)
        .unwrap()
}

type OriginCall = fn(&Origin) -> Result<(), KernelError>;

#[test]
#[ignore = "manual finite installed-origin CPU screen; not admission or a live preparation retry"]
fn fresh_fd_identity_original_origin_cpu_screen() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let cohort: usize = std::env::var("HC074_FD_IDENTITY_COHORT")
        .unwrap()
        .parse()
        .unwrap();
    assert!(cohort < 8);
    let origin = Origin::open().unwrap();
    let functions: [OriginCall; 3] = [reference_origin, reference_origin, Origin::revalidate];
    let labels = ["reference", "reference-control", "fstat-installed"];
    let orders = [
        [0, 1, 2],
        [1, 2, 0],
        [2, 0, 1],
        [0, 2, 1],
        [2, 1, 0],
        [1, 0, 2],
    ];
    for call in functions {
        for _ in 0..32 {
            call(&origin).unwrap();
        }
    }
    for sample in 0..12 {
        for (slot, variant) in orders[(sample + cohort) % 6].into_iter().enumerate() {
            let call = black_box(functions[variant]);
            let start_cpu = cpu();
            let start = std::time::Instant::now();
            for _ in 0..1024 {
                black_box(call(black_box(&origin))).unwrap();
            }
            let wall_ns = start.elapsed().as_nanos();
            let cpu_ns = cpu().checked_sub(start_cpu).unwrap();
            println!("HC074-ORIGIN-IDENTITY cohort={cohort} sample={sample} slot={slot} variant={} calls=1024 cpu_ns={cpu_ns} wall_ns={wall_ns}", labels[variant]);
        }
    }
}
