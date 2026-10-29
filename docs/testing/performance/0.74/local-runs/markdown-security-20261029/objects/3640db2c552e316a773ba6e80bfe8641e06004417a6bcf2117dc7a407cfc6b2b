//! Standalone fresh-FD identity screen, not a guard, cache or admission gate.
extern crate libc;

use std::fs::{File, OpenOptions};
use std::hint::black_box;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::time::Instant;

type Identity = (u64, u64);
type Observe = fn(&File) -> io::Result<Identity>;

fn reference(file: &File) -> io::Result<Identity> {
    let metadata = file.metadata()?;
    Ok((metadata.dev(), metadata.ino()))
}

fn candidate(file: &File) -> io::Result<Identity> {
    let mut value: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: live borrowed descriptor and writable correctly sized stat object.
    if unsafe { libc::fstat(file.as_raw_fd(), &mut value) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((value.st_dev, value.st_ino))
}

fn cpu() -> u64 {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    // SAFETY: fixed clock query and writable timespec, never a time adjustment.
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

fn main() {
    assert_eq!(std::env::consts::ARCH, "x86_64");
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 2);
    let cohort: usize = args[1].parse().unwrap();
    assert!(cohort < 8);
    let paths = [
        "/proc",
        "/proc/thread-self/ns/user",
        "/proc/thread-self/ns/mnt",
        "/proc/thread-self/ns/pid",
        "/proc/thread-self/ns/time",
        "/dev/null",
    ];
    let mut files: Vec<_> = paths.into_iter().map(|p| File::open(p).unwrap()).collect();
    let path = std::env::temp_dir().join(format!("fresh-identity-{}-{cohort}", std::process::id()));
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
    let functions: [Observe; 3] = [reference, reference, candidate];
    let labels = ["reference", "reference-control", "fstat-prototype"];
    let orders = [
        [0, 1, 2],
        [1, 2, 0],
        [2, 0, 1],
        [0, 2, 1],
        [2, 1, 0],
        [1, 0, 2],
    ];
    println!("HC074-FD-IDENTITY-ORIGIN cohort={cohort} files={} iterations=8192 samples=12 product=false admission=false", files.len());
    for sample in 0..12 {
        for (slot, variant) in orders[(sample + cohort) % orders.len()]
            .into_iter()
            .enumerate()
        {
            let call = black_box(functions[variant]);
            let start_cpu = cpu();
            let start_wall = Instant::now();
            for _ in 0..8192 {
                for file in &files {
                    black_box(call(black_box(file))).unwrap();
                }
            }
            let wall = start_wall.elapsed().as_nanos();
            let cpu_ns = cpu().checked_sub(start_cpu).unwrap();
            println!("HC074-FD-IDENTITY cohort={cohort} sample={sample} slot={slot} variant={} calls={} cpu_ns={cpu_ns} wall_ns={wall}", labels[variant], 8192*files.len());
        }
    }
    for file in &files {
        assert_eq!(reference(file).unwrap(), candidate(file).unwrap());
    }
    drop(files);
    std::fs::remove_file(path).unwrap();
}
