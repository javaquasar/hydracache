//! Local fresh observations; no authorization clock, cached value or start path.
use super::*;
use crate::diagnostic_ipc::validation_trace::kernel_walk;

fn outcome<T>(result: Result<T, KernelError>) -> String {
    match result {
        Ok(_) => "ok".into(),
        Err(KernelError::Io(e)) => format!("io:{:?}:{:?}", e.kind(), e.raw_os_error()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn fresh_status_openers_preserve_identity_flags_and_refusals() {
    let root = Origin::open().unwrap();
    let parent = child(&root.root, &std::process::id().to_string(), true).unwrap();
    let old = child(&parent, "status", false).unwrap();
    let new = fixed_child(&parent, c"status", false).unwrap();
    let identity = |file: &File| {
        let m = file.metadata().unwrap();
        (m.dev(), m.ino(), m.mode())
    };
    assert_eq!(identity(&old), identity(&new));
    for op in [libc::F_GETFL, libc::F_GETFD] {
        assert_eq!(unsafe { libc::fcntl(old.as_raw_fd(), op) }, unsafe {
            libc::fcntl(new.as_raw_fd(), op)
        });
    }
    let scratch = tempfile::tempdir().unwrap();
    let temp = File::open(scratch.path()).unwrap();
    for parent in [&root.root, &old, &temp] {
        assert_eq!(
            outcome(child(parent, "status", false)),
            outcome(fixed_child(parent, c"status", false))
        );
    }
    std::os::unix::fs::symlink("/proc/self/status", scratch.path().join("status")).unwrap();
    assert_eq!(
        outcome(child(&temp, "status", false)),
        outcome(fixed_child(&temp, c"status", false))
    );
    std::fs::remove_file(scratch.path().join("status")).unwrap();
    std::fs::write(scratch.path().join("status"), b"not proc").unwrap();
    assert_eq!(
        outcome(child(&temp, "status", false)),
        outcome(fixed_child(&temp, c"status", false))
    );
    root.revalidate().unwrap();
}

#[test]
fn fresh_status_observation_does_not_reuse_previous_bytes() {
    let file = tempfile::tempfile().unwrap();
    file.write_at(b"old", 0).unwrap();
    assert_eq!(bounded(&file).unwrap(), b"old");
    file.write_at(b"new", 0).unwrap();
    assert_eq!(bounded(&file).unwrap(), b"new");
    file.set_len(0).unwrap();
    assert!(bounded(&file).is_err());
    file.set_len((DOC_BYTES + 1) as u64).unwrap();
    assert!(bounded(&file).is_err());
}

#[test]
fn fresh_status_runtime_uses_the_measured_fixed_openers() {
    let source = include_str!("../../src/diagnostic_kernel_admission.rs");
    for exact in [
        "let status = origin::fixed_child(&directory, c\"status\", false)?;",
        "let named = origin::fixed_child(&self.directory, c\"status\", false)?;",
    ] {
        assert_eq!(source.matches(exact).count(), 1, "unimplemented candidate");
    }
}

fn cpu() -> Option<u64> {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut value) } != 0 {
        return None;
    }
    u64::try_from(value.tv_sec)
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(u64::try_from(value.tv_nsec).ok()?)
}

#[test]
#[ignore = "manual local nonroot fresh kernel observation attribution; no start authority"]
fn fresh_kernel_observation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    let credentials = libc::ucred {
        pid: unsafe { libc::getpid() },
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
    };
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, credentials.pid, 0) };
    assert!(fd >= 0);
    // SAFETY: a successful pidfd_open returns a newly owned descriptor.
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd as i32) };
    let sender = Sender::pin(&origin, credentials, pidfd).unwrap();
    for sample in 0..3 {
        for phase in [
            "origin",
            "sender",
            "pidfd",
            "status-document",
            "status-metadata",
            "status-filesystem",
        ] {
            let started = std::time::Instant::now();
            let before = cpu();
            let ((result, event), report) = kernel_walk::capture(|| {
                super::super::refusal_trace::capture(|| {
                    for _ in 0..64 {
                        match phase {
                            "origin" => origin.revalidate()?,
                            "sender" => sender.revalidate(&origin)?,
                            "pidfd" => alive(&sender.pidfd, sender.credentials.pid)?,
                            "status-document" => {
                                std::hint::black_box(bounded(&sender.status)?);
                            }
                            "status-metadata" => {
                                std::hint::black_box(sender.status.metadata()?);
                            }
                            "status-filesystem" => filesystem(&sender.status, 0x9fa0)?,
                            _ => unreachable!(),
                        }
                    }
                    Ok::<_, KernelError>(())
                })
            });
            let cpu_ns = before.and_then(|b| cpu()?.checked_sub(b));
            let success = result.is_ok();
            let nodes: Vec<_> = report.nodes.iter().map(|n| serde_json::json!({"calls":n.calls,"inclusive_ns":n.inclusive_ns,"outside_children_ns":n.outside_children_ns})).collect();
            println!(
                "HC074-FRESH-KERNEL {}",
                serde_json::json!({"sample":sample,"phase":phase,"iterations":64,"wall_ns":started.elapsed().as_nanos(),"process_cpu_ns":cpu_ns,"nodes":nodes,"roots":report.roots,"outcome":outcome(result),"first_refusal":event.as_ref().map(|e|e.site),"guard_clock_added":false,"deadline_changed":false,"server":false,"product":false})
            );
            assert!(success);
            assert!(event.is_none());
            let expected = match phase {
                "origin" => [0, 0, 0, 0, 64, 128, 256, 0, 0, 128],
                "sender" => [0, 64, 128, 128, 128, 256, 512, 0, 0, 512],
                "pidfd" => [0, 0, 64, 0, 0, 0, 0, 0, 0, 64],
                "status-document" => [0, 0, 0, 0, 0, 0, 0, 0, 0, 64],
                _ => [0; 10],
            };
            assert_eq!(report.nodes.map(|n| n.calls), expected);
            for i in 0..kernel_walk::COUNT {
                assert_eq!(
                    report.nodes[i].calls,
                    report.roots[i] + report.edges.iter().map(|r| r[i].calls).sum::<u64>()
                );
                assert_eq!(
                    report.nodes[i].inclusive_ns - report.nodes[i].outside_children_ns,
                    report.edges[i].iter().map(|e| e.inclusive_ns).sum::<u128>()
                );
            }
        }
    }
    origin.revalidate().unwrap();
}
