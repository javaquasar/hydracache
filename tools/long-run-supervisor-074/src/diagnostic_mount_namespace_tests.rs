//! Owned local processes and private modeled drift, not foreign namespace mutation.
use super::*;
use crate::diagnostic_process::pin_owned_test_helper;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};

fn identity() -> super::FileId {
    super::FileId(1, 2)
}
struct Owned(Child);
impl Owned {
    fn new() -> Self {
        Self(
            Command::new("/bin/cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
    fn finish(&mut self) {
        self.0.stdin.take();
        self.0.wait().unwrap();
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.finish();
    }
}
fn refused(read: &mut SameMountNamespaceRead<'_>) {
    assert!(read.is_refused());
    assert!(matches!(
        read.revalidate(),
        Err(MountNamespaceError::Refused)
    ));
}
#[test]
fn mount_namespace_order_brackets_both_observations_with_generation() {
    let mut gate = Gate::new(identity());
    let mut calls = vec![];
    gate.observe(true, |step| {
        calls.push(step);
        Ok(identity())
    })
    .unwrap();
    assert_eq!(
        calls,
        [
            Step::Generation,
            Step::Namespace,
            Step::Generation,
            Step::Namespace,
            Step::Generation
        ]
    );
}
#[test]
fn mount_namespace_every_first_error_stops_with_original_type() {
    for failed in 0..5 {
        let mut gate = Gate::new(identity());
        let mut calls = 0;
        assert!(matches!(
            gate.observe(true, |_| {
                let here = calls;
                calls += 1;
                if here == failed {
                    Err(MountNamespaceError::Process(super::ProcessError::NotLive))
                } else {
                    Ok(identity())
                }
            }),
            Err(MountNamespaceError::Process(super::ProcessError::NotLive))
        ));
        assert_eq!(calls, failed + 1);
        assert!(matches!(
            gate.observe(true, |_| panic!("no IO")),
            Err(MountNamespaceError::Refused)
        ));
    }
}
#[test]
fn mount_namespace_prior_refusal_and_foreign_thread_never_observe() {
    let mut gate = Gate::new(identity());
    assert!(matches!(
        gate.observe(false, |_| panic!("thread mismatch precedes IO")),
        Err(MountNamespaceError::Drift)
    ));
    assert!(matches!(
        gate.observe(true, |_| panic!("no restore")),
        Err(MountNamespaceError::Refused)
    ));
}
#[test]
fn mount_namespace_identity_drift_never_adopts_device_or_inode() {
    for changed in [super::FileId(3, 2), super::FileId(1, 3)] {
        let mut gate = Gate::new(identity());
        assert!(matches!(
            gate.observe(true, |_| Ok(changed)),
            Err(MountNamespaceError::Drift)
        ));
        assert_eq!(gate.original, identity());
        assert!(matches!(
            gate.observe(true, |_| Ok(identity())),
            Err(MountNamespaceError::Refused)
        ));
    }
}
#[test]
fn mount_namespace_seeded_failure_and_identity_are_sticky() {
    let seed = 0x7582026_u64;
    eprintln!("mount namespace mutation seed={seed:#x}; mutations=256");
    let mut rng = seed;
    for _ in 0..256 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut gate = Gate::new(identity());
        gate.observe(true, |_| Ok(identity())).unwrap();
        let failed = rng as usize % 5;
        let mut calls = 0;
        let error = gate.observe(true, |_| {
            let here = calls;
            calls += 1;
            if here == failed {
                Err(MountNamespaceError::Invalid)
            } else {
                Ok(identity())
            }
        });
        assert!(matches!(error, Err(MountNamespaceError::Invalid)));
        assert_eq!(calls, failed + 1);
        assert_eq!(gate.original, identity());
        assert!(matches!(
            gate.observe(true, |_| panic!("no retry")),
            Err(MountNamespaceError::Refused)
        ));
        let mut identity_gate = Gate::new(identity());
        let changed = if rng & 1 == 0 {
            super::FileId(1 + (rng % 65_536) + 1, 2)
        } else {
            super::FileId(1, 2 + (rng % 65_536) + 1)
        };
        assert!(matches!(
            identity_gate.observe(true, |_| Ok(changed)),
            Err(MountNamespaceError::Drift)
        ));
        assert_eq!(identity_gate.original, identity());
        assert!(matches!(
            identity_gate.observe(true, |_| panic!("no identity refresh")),
            Err(MountNamespaceError::Refused)
        ));
    }
}
#[test]
fn mount_namespace_guard_is_neither_send_nor_sync() {
    trait AmbiguousSend<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSend<()> for T {}
    impl<T: ?Sized + Send> AmbiguousSend<u8> for T {}
    let _ = <SameMountNamespaceRead<'static> as AmbiguousSend<_>>::marker;
    trait AmbiguousSync<A> {
        fn marker() {}
    }
    impl<T: ?Sized> AmbiguousSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousSync<u8> for T {}
    let _ = <SameMountNamespaceRead<'static> as AmbiguousSync<_>>::marker;
}
#[test]
fn mount_namespace_owned_roundtrip_readonly_flags_and_healthy_drop() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    {
        let mut read = pin_same_mount_namespace(&process).unwrap();
        for _ in 0..8 {
            read.revalidate().unwrap();
        }
        for f in [
            &read.probe.worker,
            &read.probe.observer,
            &read.probe.directory,
        ] {
            // SAFETY: live borrowed descriptors, scalar nonmutating fcntl queries.
            assert_eq!(
                unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETFL) } & libc::O_ACCMODE,
                libc::O_RDONLY
            );
            assert_ne!(
                unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        assert!(!read.is_refused());
    }
    process.revalidate().unwrap();
}
#[test]
fn mount_namespace_owned_exit_refuses_despite_retained_namespace() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut read = pin_same_mount_namespace(&process).unwrap();
    child.finish();
    assert!(namespace_id(&read.probe.worker).is_ok());
    assert!(matches!(
        read.revalidate(),
        Err(MountNamespaceError::Process(_))
    ));
    refused(&mut read);
}
#[test]
fn mount_namespace_constructor_refuses_dead_original_generation() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    child.finish();
    assert!(matches!(
        pin_same_mount_namespace(&process),
        Err(MountNamespaceError::Process(_))
    ));
}
#[test]
fn mount_namespace_wrong_objects_and_restoration_latch() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    for observer in [false, true] {
        for wrong in ["/dev/null", "/proc/thread-self/ns/user"] {
            let mut read = pin_same_mount_namespace(&process).unwrap();
            let slot = if observer {
                &mut read.probe.observer
            } else {
                &mut read.probe.worker
            };
            let original = std::mem::replace(slot, File::open(wrong).unwrap());
            assert!(matches!(
                read.revalidate(),
                Err(MountNamespaceError::Invalid)
            ));
            if observer {
                read.probe.observer = original;
            } else {
                read.probe.worker = original;
            }
            refused(&mut read);
            process.revalidate().unwrap();
        }
    }
}
#[test]
fn mount_namespace_original_directory_substitution_cannot_restore() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut read = pin_same_mount_namespace(&process).unwrap();
    let original = std::mem::replace(&mut read.probe.directory, File::open("/proc").unwrap());
    assert!(matches!(read.revalidate(), Err(MountNamespaceError::Drift)));
    read.probe.directory = original;
    refused(&mut read);
}
#[test]
fn mount_namespace_named_directory_pin_drift_cannot_refresh() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut read = pin_same_mount_namespace(&process).unwrap();
    let original = read.probe.directory_id;
    read.probe.directory_id = identity();
    assert!(matches!(read.revalidate(), Err(MountNamespaceError::Drift)));
    read.probe.directory_id = original;
    refused(&mut read);
}
#[test]
fn mount_namespace_type_is_kernel_checked_not_link_text() {
    for wrong in [
        "/proc/thread-self/ns/user",
        "/dev/null",
        "/proc/self/status",
    ] {
        assert!(matches!(
            namespace_id(&File::open(wrong).unwrap()),
            Err(MountNamespaceError::Invalid)
        ));
    }
    let file = File::open("/proc/thread-self/ns/mnt").unwrap();
    assert!(namespace_id(&file).is_ok());
    assert!(matches!(
        namespace_id(&tempfile::tempfile().unwrap()),
        Err(MountNamespaceError::Invalid)
    ));
}
#[test]
fn mount_namespace_observer_tid_drift_precedes_dead_process_and_bad_fd() {
    let mut child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut read = pin_same_mount_namespace(&process).unwrap();
    let original = read.probe.observer_tid;
    read.probe.observer_tid = 0;
    read.probe.worker = File::open("/dev/null").unwrap();
    child.finish();
    assert!(matches!(read.revalidate(), Err(MountNamespaceError::Drift)));
    read.probe.observer_tid = original;
    refused(&mut read);
}
#[test]
fn mount_namespace_distinct_identity_check_is_not_initial_host_attestation() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    let mut read = pin_same_mount_namespace(&process).unwrap();
    let original = read.probe.original;
    read.probe.original = super::FileId(original.0, original.1 + 1);
    assert!(matches!(read.revalidate(), Err(MountNamespaceError::Drift)));
    read.probe.original = original;
    refused(&mut read);
}
#[test]
fn mount_namespace_concurrent_readers_construct_on_own_threads() {
    let child = Owned::new();
    let process = pin_owned_test_helper(child.0.id());
    std::thread::scope(|scope| {
        for index in 0..4 {
            let process = &process;
            scope.spawn(move || {
                let mut read = pin_same_mount_namespace(process).unwrap();
                if index == 0 {
                    read.probe.worker = File::open("/dev/null").unwrap();
                    assert!(read.revalidate().is_err());
                    refused(&mut read);
                } else {
                    for _ in 0..8 {
                        read.revalidate().unwrap();
                    }
                    assert!(!read.is_refused());
                }
            });
        }
    });
}
