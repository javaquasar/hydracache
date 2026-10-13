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
pub(super) fn filesystem(file: &File, magic: libc::c_long) -> Result<(), KernelError> {
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut stat) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.f_type != magic {
        return Err(KernelError::Invalid);
    }
    Ok(())
}
pub(super) fn child(parent: &File, name: &str, directory: bool) -> Result<File, KernelError> {
    let name = CString::new(name).map_err(|_| KernelError::Invalid)?;
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
    // These fixed kernel magic links must be followed, then checked as nsfs.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(format!("/proc/thread-self/ns/{name}"))?;
    filesystem(&file, 0x6e736673)?;
    let m = file.metadata()?;
    Ok((file, (m.dev(), m.ino())))
}
fn boot(root: &File) -> Result<String, KernelError> {
    let sys = child(root, "sys", true)?;
    let kernel = child(&sys, "kernel", true)?;
    let random = child(&kernel, "random", true)?;
    let raw = bounded(&child(&random, "boot_id", false)?)?;
    if raw.len() != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(KernelError::Invalid);
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| KernelError::Invalid)
}
fn clock(id: libc::clockid_t) -> Result<u64, KernelError> {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(id, &mut value) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    timespec(value.tv_sec, value.tv_nsec)
}
pub(super) fn timespec(sec: libc::time_t, ns: libc::c_long) -> Result<u64, KernelError> {
    if sec < 0 || !(0..1_000_000_000).contains(&ns) {
        return Err(KernelError::Invalid);
    }
    (sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|s| s.checked_add(ns as u64))
        .ok_or(KernelError::Invalid)
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
        let named = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/proc")?;
        for file in [&named, &self.root] {
            filesystem(file, 0x9fa0)?;
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != self.id || boot(file)? != self.boot {
                return Err(KernelError::Invalid);
            }
        }
        for (index, (file, id)) in self.namespaces.iter().enumerate() {
            filesystem(file, 0x6e736673)?;
            let m = file.metadata()?;
            if (m.dev(), m.ino()) != *id || namespace(NAMES[index])?.1 != *id {
                return Err(KernelError::Invalid);
            }
        }
        Ok(())
    }
    pub(super) fn sample(&self) -> Result<KernelTimePoint, KernelError> {
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
    pub(super) fn child(&self, name: &str, directory: bool) -> Result<File, KernelError> {
        child(&self.root, name, directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
