//! Synthetic role data with real private kernel resources, NOT original authority.
use crate::bootstrap_child_material as entry_material;
use hydracache_long_run_supervisor_074::diagnostic_builder::bootstrap_build::entry_format::AssertedEntry;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    process::{Child, Command, Stdio},
};
pub fn sealed(raw: &[u8]) -> File {
    entry_material::sealed(raw, entry_material::SEALS)
}
fn owned(fd: i32) -> File {
    assert!(fd >= 0);
    unsafe { File::from_raw_fd(fd) }
}
fn pair(file: &File) -> [u64; 2] {
    let m = file.metadata().unwrap();
    [m.dev(), m.ino()]
}
pub fn identity(file: &File) -> [u64; 3] {
    let m = file.metadata().unwrap();
    [m.dev(), m.ino(), m.len()]
}
fn pipe() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
        0
    );
    fds.map(owned)
}
fn socket() -> [File; 2] {
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    let files = fds.map(owned);
    let yes = 1i32;
    for f in &files {
        assert_eq!(
            unsafe {
                libc::setsockopt(
                    f.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PASSCRED,
                    (&yes as *const i32).cast(),
                    4,
                )
            },
            0
        );
    }
    files
}
fn image(marker: u8) -> Vec<u8> {
    let mut bytes = vec![marker; 128];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    for (at, v) in [(16, 3u16), (18, 62), (52, 64), (54, 56), (56, 1)] {
        bytes[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
    bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
    bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
    bytes
}
pub fn now() -> u64 {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) },
        0
    );
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
pub fn arm(file: &File, deadline: u64) {
    let spec = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: libc::timespec {
            tv_sec: (deadline / 1_000_000_000) as i64,
            tv_nsec: (deadline % 1_000_000_000) as i64,
        },
    };
    assert_eq!(
        unsafe {
            libc::timerfd_settime(
                file.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME,
                &spec,
                std::ptr::null_mut(),
            )
        },
        0
    );
}
pub struct Fixture {
    pub files: [File; 15],
    pub entry: AssertedEntry,
    pub children: Vec<Child>,
    pub retained: Vec<File>,
}
impl Fixture {
    pub fn new() -> Self {
        let children: Vec<_> = (0..2)
            .map(|_| {
                Command::new("/bin/sleep")
                    .arg("30")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap()
            })
            .collect();
        // This local guard ensures child cleanup even if later fixture construction fails.
        let mut guard = Children(Some(children));
        let children = guard.0.as_ref().unwrap();
        let [stdin, writer] = pipe();
        drop(writer);
        let [out_read, stdout] = pipe();
        let [err_read, stderr] = pipe();
        let [peer, release] = socket();
        let images = [image(1), image(2), image(3), image(4)];
        let [worker, host, launcher, bootstrap_image] = images.each_ref().map(|raw| sealed(raw));
        let config_raw = [7; 64];
        let config = sealed(&config_raw);
        let owner = unsafe {
            [
                i64::from(libc::getpid()),
                libc::syscall(libc::SYS_gettid),
                i64::from(libc::getuid()),
                i64::from(libc::getgid()),
                i64::from(libc::geteuid()),
                i64::from(libc::getegid()),
            ]
        };
        let creator =
            owned(unsafe { libc::syscall(libc::SYS_pidfd_open, owner[0] as i32, 0u32) } as i32);
        let guardians: [File; 2] = std::array::from_fn(|i| {
            owned(
                unsafe { libc::syscall(libc::SYS_pidfd_open, children[i].id() as i32, 0u32) }
                    as i32,
            )
        });
        let issued = now();
        let deadline = issued + 10_000_000_000;
        let alarm = owned(unsafe {
            libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
        });
        arm(&alarm, deadline);
        let streams = [pair(&stdin), pair(&stdout), pair(&stderr), pair(&release)];
        let mut legacy = [0; 256];
        legacy[..8].copy_from_slice(b"HCHL0741");
        legacy[8..16].copy_from_slice(&deadline.to_le_bytes());
        legacy[40..72].fill(8);
        for (at, v) in [
            (16, owner[0] as u32),
            (20, owner[4] as u32),
            (24, owner[5] as u32),
            (28, children[0].id()),
            (32, children[1].id()),
        ] {
            legacy[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        let hashes: [[u8; 32]; 5] = [
            Sha256::digest(&images[1]).into(),
            Sha256::digest(&images[2]).into(),
            Sha256::digest(&images[3]).into(),
            Sha256::digest(&images[0]).into(),
            Sha256::digest(config_raw).into(),
        ];
        legacy[72..104].copy_from_slice(&hashes[3]);
        legacy[104..136].copy_from_slice(&hashes[4]);
        for (at, words) in [
            (136, identity(&worker).to_vec()),
            (160, identity(&config).to_vec()),
            (184, streams[0].to_vec()),
            (200, streams[1].to_vec()),
            (216, streams[2].to_vec()),
            (232, streams[3].to_vec()),
        ] {
            for (i, v) in words.iter().enumerate() {
                legacy[at + i * 8..at + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
            }
        }
        let legacy_file = sealed(&legacy);
        let template = AssertedEntry::decode(&entry_material::raw()).unwrap();
        let entry = AssertedEntry {
            owner: owner.map(|v| v as u64),
            origins: [issued; 4],
            bounds: [
                issued + 300_000_000_000,
                issued + 60_000_000_000,
                deadline,
                deadline,
            ],
            attempt: template.attempt,
            original_context: template.original_context,
            root: [1, 2, deadline],
            serial: template.serial,
            names: template.names,
            documents: template.documents,
            identities: [
                identity(&host),
                identity(&launcher),
                identity(&bootstrap_image),
                identity(&worker),
                identity(&config),
                identity(&legacy_file),
            ],
            streams,
            guardians: std::array::from_fn(|i| {
                [
                    u64::from(children[i].id()),
                    pair(&guardians[i])[0],
                    pair(&guardians[i])[1],
                ]
            }),
            image_hashes: hashes,
            legacy_hash: Sha256::digest(legacy).into(),
            census_hash: [5; 32],
            state_hash: [6; 32],
        };
        let entry_file = sealed(&entry.encode().unwrap());
        let [a, b] = guardians;
        let files = [
            stdin,
            stdout,
            stderr,
            release,
            worker,
            config,
            creator,
            a,
            b,
            alarm,
            legacy_file,
            host,
            launcher,
            bootstrap_image,
            entry_file,
        ];
        for f in &files {
            assert_eq!(unsafe { libc::fcntl(f.as_raw_fd(), libc::F_SETFD, 0) }, 0);
        }
        Self {
            files,
            entry,
            children: guard.0.take().unwrap(),
            retained: vec![peer, out_read, err_read],
        }
    }
    pub fn fds(&self) -> [i32; 15] {
        self.files.each_ref().map(|f| f.as_raw_fd())
    }
    pub fn replace_entry(&mut self) {
        self.files[14] = sealed(&self.entry.encode().unwrap());
    }
}
struct Children(Option<Vec<Child>>);
impl Drop for Children {
    fn drop(&mut self) {
        if let Some(children) = self.0.as_mut() {
            for child in children {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
