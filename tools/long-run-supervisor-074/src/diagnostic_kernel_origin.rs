use super::*;
use std::ffi::CString;
use std::os::fd::FromRawFd;

#[derive(Debug, Clone)]
pub struct KernelTimePoint {
    boot: String,
    boottime: u64,
    pub(super) realtime_ns: u64,
}
pub(super) type TimePoint = KernelTimePoint;
impl KernelTimePoint {
    pub fn boot_id(&self) -> &str {
        &self.boot
    }
    pub fn boottime_ns(&self) -> u64 {
        self.boottime
    }
    pub fn unix_seconds(&self) -> u64 {
        self.realtime_ns / 1_000_000_000
    }
}
pub(super) struct Origin {
    root: File,
    id: (u64, u64),
    boot: String,
    namespaces: [(File, (u64, u64)); 4],
}
const NAMES: [&str; 4] = ["user", "mnt", "pid", "time"];

#[path = "diagnostic_kernel_manager_binding.rs"]
mod manager_binding;
pub(crate) use manager_binding::OriginalReceiverBinding;
pub(super) fn filesystem(file: &File, magic: libc::c_long) -> Result<(), KernelError> {
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type != magic {
        return Err(kernel_invalid!("filesystem-type"));
    }
    Ok(())
}
pub(super) fn child(parent: &File, name: &str, directory: bool) -> Result<File, KernelError> {
    let name = CString::new(name).map_err(|_| kernel_invalid!("proc-child-name"))?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    filesystem(&file, 0x9fa0)?;
    Ok(file)
}
fn namespace(name: &str) -> Result<(File, (u64, u64)), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Namespace,
    );
    // These fixed kernel magic links must be followed, then checked as nsfs.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(namespace_path(name).as_ref())?;
    filesystem(&file, 0x6e736673)?;
    let m = file.metadata()?;
    Ok((file, (m.dev(), m.ino())))
}
#[path = "diagnostic_boot_document.rs"]
mod boot_document;
fn boot(root: &File) -> Result<String, KernelError> {
    boot_apply(root, boot_value)
}
#[path = "diagnostic_boot_comparison.rs"]
mod boot_comparison;
fn boot_apply<T>(
    root: &File,
    decode: impl FnOnce(([u8; 38], usize)) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = fixed_child(root, c"sys", true)?;
    let kernel = fixed_child(&sys, c"kernel", true)?;
    let random = fixed_child(&kernel, c"random", true)?;
    let file = fixed_child(&random, c"boot_id", false)?;
    let raw = boot_read(|bytes| file.read_at(bytes, 0))?;
    // As before, close the proc document before decoding the owned bytes.
    drop(file);
    decode(raw)
}

pub(super) fn fixed_child(
    parent: &File,
    name: &std::ffi::CStr,
    directory: bool,
) -> Result<File, KernelError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    filesystem(&file, 0x9fa0)?;
    Ok(file)
}
fn namespace_path(name: &str) -> std::borrow::Cow<'static, str> {
    use std::borrow::Cow;
    match name {
        "user" => Cow::Borrowed("/proc/thread-self/ns/user"),
        "mnt" => Cow::Borrowed("/proc/thread-self/ns/mnt"),
        "pid" => Cow::Borrowed("/proc/thread-self/ns/pid"),
        "time" => Cow::Borrowed("/proc/thread-self/ns/time"),
        _ => Cow::Owned(format!("/proc/thread-self/ns/{name}")),
    }
}
fn boot_read(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> Result<([u8; 38], usize), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,
    );
    Ok(boot_document::read_once(read)?)
}
fn boot_value(raw: ([u8; 38], usize)) -> Result<String, KernelError> {
    boot_view(&raw).map(str::to_owned)
}
fn boot_view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {
    let (raw, count) = raw;
    let count = *count;
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    std::str::from_utf8(&raw[..36]).map_err(|_| kernel_invalid!("boot-utf8"))
}
fn clock(id: libc::clockid_t) -> Result<u64, KernelError> {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(id, &mut value) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    timespec(value.tv_sec, value.tv_nsec)
}
fn require_namespace(
    file: &File,
    expected: (u64, u64),
    kind: libc::c_int,
) -> Result<(), KernelError> {
    filesystem(file, libc::NSFS_MAGIC as libc::c_long)?;
    // SAFETY: retained nsfs descriptor; NS_GET_NSTYPE takes no pointer.
    if unsafe { libc::ioctl(file.as_raw_fd(), libc::NS_GET_NSTYPE) } != kind {
        return Err(kernel_invalid!("namespace-type"));
    }
    let metadata = file.metadata()?;
    if (metadata.dev(), metadata.ino()) != expected {
        return Err(kernel_invalid!("namespace-object"));
    }
    Ok(())
}
pub(super) fn timespec(sec: libc::time_t, ns: libc::c_long) -> Result<u64, KernelError> {
    if sec < 0 || !(0..1_000_000_000).contains(&ns) {
        return Err(kernel_invalid!("clock-timespec"));
    }
    kernel_required!(
        (sec as u64)
            .checked_mul(1_000_000_000)
            .and_then(|s| s.checked_add(ns as u64)),
        "clock-overflow"
    )
}
impl Origin {
    pub(super) fn open() -> Result<Self, KernelError> {
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/proc")?;
        filesystem(&root, 0x9fa0)?;
        let m = root.metadata()?;
        let boot = boot(&root)?;
        let namespaces = NAMES.map(namespace);
        let [user, mount, pid, time] = namespaces;
        let origin = Self {
            root,
            id: (m.dev(), m.ino()),
            boot,
            namespaces: [user?, mount?, pid?, time?],
        };
        origin.revalidate()?;
        Ok(origin)
    }
    pub(super) fn revalidate(&self) -> Result<(), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Origin,
        );
        let named = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/proc")?;
        for file in [&named, &self.root] {
            filesystem(file, 0x9fa0)?;
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.id || !boot_comparison::matches(file, &self.boot)? {
                return Err(kernel_invalid!("proc-origin-or-boot"));
            }
        }
        for (index, (file, id)) in self.namespaces.iter().enumerate() {
            filesystem(file, 0x6e736673)?;
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != *id || namespace(NAMES[index])?.1 != *id {
                return Err(kernel_invalid!("namespace-origin"));
            }
        }
        Ok(())
    }
    pub(super) fn sample(&self) -> Result<KernelTimePoint, KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sample,
        );
        self.revalidate()?;
        let boottime = clock(libc::CLOCK_BOOTTIME)?;
        let realtime_ns = clock(libc::CLOCK_REALTIME)?;
        self.revalidate()?;
        Ok(KernelTimePoint {
            boot: self.boot.clone(),
            boottime,
            realtime_ns,
        })
    }
    pub(super) fn require_worker_context(
        &self,
        expected: &crate::diagnostic_worker_policy::AssertedWorkerHost,
    ) -> Result<(), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::WorkerOrigin,
        );
        self.revalidate()?;
        if self.boot != expected.boot_id {
            return Err(kernel_invalid!("worker-boot"));
        }
        for (index, (identity, kind)) in [
            (expected.user_namespace, libc::CLONE_NEWUSER),
            (expected.mount_namespace, libc::CLONE_NEWNS),
        ]
        .into_iter()
        .enumerate()
        {
            let (file, retained) = &self.namespaces[index];
            if *retained != (identity.device, identity.inode) {
                return Err(kernel_invalid!("worker-namespace"));
            }
            require_namespace(file, *retained, kind)?;
        }
        self.revalidate()
    }
    pub(super) fn child(&self, name: &str, directory: bool) -> Result<File, KernelError> {
        child(&self.root, name, directory)
    }
}

#[cfg(test)]
#[path = "diagnostic_boot_buffer_tests.rs"]
mod boot_buffer_tests;

#[cfg(test)]
#[path = "../tests/support/fresh_kernel_cost.rs"]
mod fresh_kernel_cost;

#[cfg(test)]
#[path = "diagnostic_fixed_leaf_path_tests.rs"]
mod fixed_leaf_path_tests;

#[cfg(test)]
#[path = "diagnostic_boot_comparison_tests.rs"]
mod boot_comparison_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kernel_refusal_capture_seeded_clock_and_namespace_classification() {
        use crate::diagnostic_kernel_admission::refusal_trace::capture;
        let mut seed = 0x0740_c096_u64;
        let mut order = [0, 1, 2, 3, 4, 5];
        for i in (1..order.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            order.swap(i, seed as usize % (i + 1));
        }
        let before = KernelTimePoint {
            boot: "original".into(),
            boottime: 10,
            realtime_ns: 100,
        };
        for case in order {
            let mut after = before.clone();
            match case {
                0 => after.boot = "changed".into(),
                1 => after.boottime -= 1,
                2 => after.realtime_ns -= 1,
                3 => after.boottime += HOLD_NS,
                4 => after.boottime += HOLD_NS - 1,
                _ => (),
            }
            let (result, event) = capture(|| valid_hold(&before, &after));
            if case < 4 {
                assert!(matches!(result, Err(KernelError::Invalid)));
                let event = event.unwrap();
                assert_eq!(
                    event.site,
                    if case < 3 {
                        "clock-order"
                    } else {
                        "packet-hold"
                    }
                );
                assert_eq!(
                    event.clock,
                    [
                        before.boottime,
                        after.boottime,
                        before.realtime_ns,
                        after.realtime_ns,
                        u64::from(before.boot == after.boot),
                        HOLD_NS
                    ]
                );
            } else {
                result.unwrap();
                assert!(event.is_none());
            }
        }
        for (s, n, expected) in [
            (1, 0, None),
            (-1, 0, Some("clock-timespec")),
            (libc::time_t::MAX, 0, Some("clock-overflow")),
        ] {
            let (result, event) = capture(|| timespec(s, n));
            assert_eq!(result.is_ok(), expected.is_none());
            assert_eq!(event.map(|e| e.site), expected);
        }
        let (file, id) = namespace("mnt").unwrap();
        for (expected, kind, site) in [
            (id, libc::CLONE_NEWUSER, Some("namespace-type")),
            (
                (id.0, id.1 + 1),
                libc::CLONE_NEWNS,
                Some("namespace-object"),
            ),
            (id, libc::CLONE_NEWNS, None),
        ] {
            let (result, event) = capture(|| require_namespace(&file, expected, kind));
            assert_eq!(result.is_ok(), site.is_none());
            assert_eq!(event.map(|e| e.site), site);
        }
        let mut origin = Origin::open().unwrap();
        origin.id.1 += 1;
        let (result, event) = capture(|| origin.revalidate());
        assert!(result.is_err());
        assert_eq!(event.unwrap().site, "proc-origin-or-boot");
        println!("kernel_refusal_classification seed=0x0740c096 clock_cases=6 namespace_cases=3 historical_cause_proved=false");
    }
    #[test]
    fn kernel_worker_context_namespace_type_and_object_are_independent_checks() {
        let (file, id) = namespace("mnt").unwrap();
        require_namespace(&file, id, libc::CLONE_NEWNS).unwrap();
        assert!(require_namespace(&file, id, libc::CLONE_NEWUSER).is_err());
        assert!(require_namespace(&file, (id.0, id.1 + 1), libc::CLONE_NEWNS).is_err());
        assert!(require_namespace(&tempfile::tempfile().unwrap(), id, libc::CLONE_NEWNS).is_err());
    }
    #[test]
    fn kernel_worker_context_original_typed_namespaces_and_drift_refuse() {
        for case in 0..7 {
            let mut origin = Origin::open().unwrap();
            let host = crate::diagnostic_worker_policy::AssertedWorkerHost {
                machine_id: "a".repeat(32),
                boot_id: origin.boot.clone(),
                user_namespace: crate::diagnostic_worker_policy::NamespaceIdentity {
                    device: origin.namespaces[0].1 .0,
                    inode: origin.namespaces[0].1 .1,
                },
                mount_namespace: crate::diagnostic_worker_policy::NamespaceIdentity {
                    device: origin.namespaces[1].1 .0,
                    inode: origin.namespaces[1].1 .1,
                },
            };
            origin.require_worker_context(&host).unwrap();
            match case {
                0 => origin.boot = "wrong".into(),
                1..=4 => origin.namespaces[case - 1].1 .1 += 1,
                5 => origin.namespaces.swap(0, 1),
                _ => origin.namespaces[0].0 = tempfile::tempfile().unwrap(),
            }
            assert!(origin.require_worker_context(&host).is_err());
        }
    }
    #[test]
    fn kernel_time_timespec_and_ordering_edges_refuse() {
        for (s, n) in [(-1, 0), (0, -1), (0, 1_000_000_000), (libc::time_t::MAX, 0)] {
            assert!(timespec(s, n).is_err());
        }
        assert_eq!(timespec(1, 999_999_999).unwrap(), 1_999_999_999);
        let point = KernelTimePoint {
            boot: "original".into(),
            boottime: 1,
            realtime_ns: 1,
        };
        for after in [
            KernelTimePoint {
                boot: "changed".into(),
                ..point.clone()
            },
            KernelTimePoint {
                boottime: 0,
                ..point.clone()
            },
            KernelTimePoint {
                realtime_ns: 0,
                ..point.clone()
            },
        ] {
            assert!(ordered(&point, &after).is_err());
        }
    }
    #[test]
    fn kernel_time_original_proc_boot_and_namespace_drift_refuse() {
        for case in 0..6 {
            let mut origin = Origin::open().unwrap();
            match case {
                0 => origin.id.1 += 1,
                1 => origin.boot = "wrong".into(),
                n => origin.namespaces[n - 2].1 .1 += 1,
            }
            assert!(origin.sample().is_err());
        }
    }
    #[test]
    fn kernel_time_packet_hold_has_exclusive_boottime_boundary() {
        let before = KernelTimePoint {
            boot: "original".into(),
            boottime: 1,
            realtime_ns: 1,
        };
        for age in [0, HOLD_NS - 1, HOLD_NS, HOLD_NS + 1] {
            let after = KernelTimePoint {
                boottime: before.boottime + age,
                ..before.clone()
            };
            assert_eq!(valid_hold(&before, &after).is_ok(), age < HOLD_NS);
        }
    }
}
