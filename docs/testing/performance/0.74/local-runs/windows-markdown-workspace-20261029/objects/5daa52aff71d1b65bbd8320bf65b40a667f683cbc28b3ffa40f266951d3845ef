use super::*;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::process::{Child, Command, Stdio};

pub(crate) struct Fixture {
    pub(crate) worker: File,
    pub(crate) pins: [File; 2],
    children: Vec<Child>,
}
struct Children(Vec<Child>);
impl Drop for Children {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let fd = unsafe {
            libc::memfd_create(
                c"hc074-census-fixture".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
            )
        };
        assert!(fd >= 0);
        let mut writer = unsafe { File::from_raw_fd(fd) };
        writer.write_all(b"worker").unwrap();
        let seals =
            libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, seals) }, 0);
        let worker = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(format!("/proc/thread-self/fd/{fd}"))
            .unwrap();
        drop(writer);
        let mut children = Children(vec![]);
        for _ in 0..2 {
            children.0.push(
                Command::new("/bin/sleep")
                    .arg("30")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap(),
            );
        }
        let pins = std::array::from_fn(|i| {
            let fd =
                unsafe { libc::syscall(libc::SYS_pidfd_open, children.0[i].id(), 0u32) } as i32;
            assert!(fd >= 0);
            unsafe { File::from_raw_fd(fd) }
        });
        Self {
            worker,
            pins,
            children: std::mem::take(&mut children.0),
        }
    }
    pub(crate) fn claims(&self) -> [(i32, [u64; 2]); 2] {
        std::array::from_fn(|i| {
            let m = self.pins[i].metadata().unwrap();
            (self.children[i].id() as i32, [m.dev(), m.ino()])
        })
    }
    pub(crate) fn files(&self) -> [(i32, &File); 2] {
        std::array::from_fn(|i| (self.children[i].id() as i32, &self.pins[i]))
    }
    pub(crate) fn census(&self) -> Census {
        let m = self.worker.metadata().unwrap();
        Census::new(
            [m.dev(), m.ino(), m.len()],
            &crate::sha256_hex(b"worker"),
            self.claims(),
            i64::from(std::process::id()),
        )
        .unwrap()
    }
    pub(crate) fn ready(&self) -> Census {
        let mut census = self.census();
        census
            .worker(&self.worker, &crate::sha256_hex(b"worker"))
            .unwrap();
        census.guardians(self.files()).unwrap();
        census
    }
    pub(crate) fn stop(&mut self, index: usize) {
        self.children[index].kill().unwrap();
        self.children[index].wait().unwrap();
    }
    pub(crate) fn original(&self) -> [i32; 20] {
        let base = self
            .worker
            .as_raw_fd()
            .max(self.pins[0].as_raw_fd())
            .max(self.pins[1].as_raw_fd())
            + 100;
        std::array::from_fn(|i| base + i as i32)
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
fn partial(original: [i32; 20]) -> [(u8, i32); 8] {
    [
        (0, 14),
        (1, 6),
        (2, 7),
        (3, 16),
        (5, 17),
        (6, 19),
        (9, 1),
        (10, 18),
    ]
    .map(|(s, i)| (s, original[i]))
}

#[test]
fn held_complete_census_records_same_worker_and_live_pidfds_without_copy_or_position_change() {
    let f = Fixture::new();
    let original = f.original();
    let before = stamp(&f.worker).unwrap();
    let a = f.ready().complete(partial(original), original).unwrap();
    let b = f.ready().complete(partial(original), original).unwrap();
    assert!(a == b);
    assert!(stamp(&f.worker).unwrap() == before);
    assert_eq!(a.roles[4], f.worker.as_raw_fd());
    assert_eq!(
        [a.roles[7], a.roles[8]],
        f.pins.each_ref().map(AsRawFd::as_raw_fd)
    );
    for (slot, fd) in partial(original) {
        assert_eq!(a.roles[slot as usize], fd);
    }
}
#[test]
fn held_complete_census_missing_duplicate_wrong_order_and_repair_after_failure_refuse() {
    let f = Fixture::new();
    let original = f.original();
    assert!(f.census().complete(partial(original), original).is_err());
    let mut c = f.census();
    c.worker(&f.worker, &crate::sha256_hex(b"worker")).unwrap();
    assert!(c.complete(partial(original), original).is_err());
    let mut c = f.census();
    assert!(c.guardians(f.files()).is_err());
    assert!(c.worker(&f.worker, &crate::sha256_hex(b"worker")).is_err());
    let mut c = f.ready();
    assert!(c.guardians(f.files()).is_err());
    assert!(c.complete(partial(original), original).is_err());
    let mut c = f.ready();
    assert!(c.worker(&f.worker, &crate::sha256_hex(b"worker")).is_err());
    assert!(c.complete(partial(original), original).is_err());
    let mut c = f.census();
    assert!(c.worker(&f.worker, &"a".repeat(64)).is_err());
    assert!(c.worker(&f.worker, &crate::sha256_hex(b"worker")).is_err());
}
#[test]
fn held_complete_census_refuses_aliases_bad_partial_slots_and_wrong_pidfd_identity() {
    let f = Fixture::new();
    let original = f.original();
    let mut negative = original;
    negative[0] = -1;
    assert!(f.ready().complete(partial(negative), negative).is_err());
    let mut duplicate = original;
    duplicate[0] = duplicate[1];
    assert!(f.ready().complete(partial(duplicate), duplicate).is_err());
    for i in 0..23 {
        let mut wrong = original;
        if i < 20 {
            wrong[i] = f.worker.as_raw_fd();
        } else {
            wrong[i - 20] = f.pins[(i - 20) % 2].as_raw_fd();
        }
        assert!(f.ready().complete(partial(wrong), wrong).is_err(), "{i}");
    }
    for i in 0..8 {
        let mut wrong = partial(original);
        wrong[i].0 = 4;
        assert!(f.ready().complete(wrong, original).is_err());
        let mut wrong = partial(original);
        wrong[i].1 += 1;
        assert!(f.ready().complete(wrong, original).is_err());
    }
    let mut c = f.census();
    c.worker(&f.worker, &crate::sha256_hex(b"worker")).unwrap();
    let mut files = f.files();
    files.swap(0, 1);
    assert!(c.guardians(files).is_err());
    assert!(c.guardians(f.files()).is_err());
    let files = f.files();
    assert!(pidfd(files[0].1, files[0].0 + 1, f.claims()[0].1).is_err());
    assert!(pidfd(&f.worker, files[0].0, [1, 2]).is_err());
    let m = f.worker.metadata().unwrap();
    assert!(pidfd(&f.worker, files[0].0, [m.dev(), m.ino()]).is_err());
}

#[test]
fn held_complete_census_invalid_claims_and_worker_shape_cannot_enter_inventory() {
    let f = Fixture::new();
    let m = f.worker.metadata().unwrap();
    let identity = [m.dev(), m.ino(), m.len()];
    let digest = crate::sha256_hex(b"worker");
    let creator = i64::from(std::process::id());
    let claims = f.claims();
    for creator in [0, 1, i64::from(claims[0].0), i64::from(claims[1].0)] {
        assert!(Census::new(identity, &digest, claims, creator).is_err());
    }
    for index in 0..2 {
        for pid in [0, 1, claims[1 - index].0] {
            let mut bad = claims;
            bad[index].0 = pid;
            assert!(Census::new(identity, &digest, bad, creator).is_err());
        }
        for at in 0..2 {
            let mut bad = claims;
            bad[index].1[at] = 0;
            assert!(Census::new(identity, &digest, bad, creator).is_err());
        }
    }
    for at in 0..3 {
        let mut bad = identity;
        bad[at] = 0;
        assert!(Census::new(bad, &digest, claims, creator).is_err());
        bad[at] = identity[at] + 1;
        let mut c = Census::new(bad, &digest, claims, creator).unwrap();
        assert!(c.worker(&f.worker, &digest).is_err());
        assert!(c.worker(&f.worker, &digest).is_err());
    }
    assert!(Census::new(identity, "bad", claims, creator).is_err());
    let writable = OpenOptions::new()
        .write(true)
        .open(format!("/proc/thread-self/fd/{}", f.worker.as_raw_fd()))
        .unwrap();
    assert!(f.census().worker(&writable, &digest).is_err());
    let mut c = f.census();
    c.worker(&f.worker, &digest).unwrap();
    assert_eq!(
        unsafe { libc::fcntl(f.pins[0].as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(c.guardians(f.files()).is_err());
    assert_eq!(
        unsafe { libc::fcntl(f.pins[0].as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    assert!(c.guardians(f.files()).is_err());
}
#[test]
fn held_complete_census_real_owned_child_exit_and_descriptor_flag_change_refuse() {
    let mut f = Fixture::new();
    let mut c = f.census();
    c.worker(&f.worker, &crate::sha256_hex(b"worker")).unwrap();
    f.stop(1);
    assert!(c.guardians(f.files()).is_err());
    assert!(c.guardians(f.files()).is_err());
    let f = Fixture::new();
    let mut c = f.census();
    assert_eq!(
        unsafe { libc::fcntl(f.worker.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(c.worker(&f.worker, &crate::sha256_hex(b"worker")).is_err());
    assert_eq!(
        unsafe { libc::fcntl(f.worker.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    assert!(c.worker(&f.worker, &crate::sha256_hex(b"worker")).is_err());
}
