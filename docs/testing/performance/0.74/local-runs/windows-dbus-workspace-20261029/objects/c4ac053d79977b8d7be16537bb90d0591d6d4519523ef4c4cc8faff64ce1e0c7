//! Full eleven-field projection prototype, never authority or a retained cache.
extern crate libc;
use std::fs::{File, OpenOptions};
use std::hint::black_box;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::time::Instant;

#[derive(Debug, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
fn reference(file: &File) -> io::Result<Stamp> {
    let m = file.metadata()?;
    Ok(Stamp {
        dev: m.dev(),
        ino: m.ino(),
        uid: m.uid(),
        gid: m.gid(),
        mode: m.mode(),
        links: m.nlink(),
        len: m.len(),
        mtime: m.mtime(),
        mtime_ns: m.mtime_nsec(),
        ctime: m.ctime(),
        ctime_ns: m.ctime_nsec(),
    })
}
fn candidate(file: &File) -> io::Result<Stamp> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: synchronous observation of a borrowed live FD into a full stat object.
    if unsafe { libc::fstat(file.as_raw_fd(), &mut m) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Stamp {
        dev: m.st_dev,
        ino: m.st_ino,
        uid: m.st_uid,
        gid: m.st_gid,
        mode: m.st_mode,
        links: m.st_nlink,
        len: m.st_size as u64,
        mtime: m.st_mtime,
        mtime_ns: m.st_mtime_nsec,
        ctime: m.st_ctime,
        ctime_ns: m.st_ctime_nsec,
    })
}
fn cpu() -> u64 {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut t) },
        0
    );
    assert!(t.tv_sec >= 0 && (0..1_000_000_000).contains(&t.tv_nsec));
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(t.tv_nsec as u64)
        .unwrap()
}
fn main() {
    assert_eq!(std::env::consts::ARCH, "x86_64");
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 2);
    let cohort: usize = args[1].parse().unwrap();
    assert!(cohort < 8);
    let mut files: Vec<_> = [
        "/proc",
        "/proc/thread-self/ns/user",
        "/proc/thread-self/ns/mnt",
        "/proc/thread-self/ns/pid",
        "/proc/thread-self/ns/time",
        "/dev/null",
    ]
    .into_iter()
    .map(|p| File::open(p).unwrap())
    .collect();
    let path = std::env::temp_dir().join(format!("fresh-stamp-{}-{cohort}", std::process::id()));
    files.push(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap(),
    );
    for file in &files {
        for _ in 0..32 {
            assert_eq!(reference(file).unwrap(), candidate(file).unwrap());
        }
    }
    let functions: [fn(&File) -> io::Result<Stamp>; 3] = [reference, reference, candidate];
    let labels = ["reference", "reference-control", "fresh-stamp"];
    let orders = [
        [0, 1, 2],
        [1, 2, 0],
        [2, 0, 1],
        [0, 2, 1],
        [2, 1, 0],
        [1, 0, 2],
    ];
    println!("HC074-STAMP-ORIGIN cohort={cohort} files=7 iterations=8192 samples=12 product=false admission=false");
    for sample in 0..12 {
        for (slot, variant) in orders[(sample + cohort) % 6].into_iter().enumerate() {
            let call = black_box(functions[variant]);
            let start_cpu = cpu();
            let start = Instant::now();
            for _ in 0..8192 {
                for file in &files {
                    black_box(call(black_box(file))).unwrap();
                }
            }
            let wall_ns = start.elapsed().as_nanos();
            let cpu_ns = cpu().checked_sub(start_cpu).unwrap();
            println!("HC074-STAMP cohort={cohort} sample={sample} slot={slot} variant={} calls=57344 cpu_ns={cpu_ns} wall_ns={wall_ns}", labels[variant]);
        }
    }
    for file in &files {
        assert_eq!(reference(file).unwrap(), candidate(file).unwrap());
    }
    drop(files);
    std::fs::remove_file(path).unwrap();
}
