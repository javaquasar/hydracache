//! Local actual-phase attribution only; never a replacement validation schedule.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::breakdown_controls::{
    Fixture, PHASES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
struct Io {
    rchar: u64,
    wchar: u64,
    syscr: u64,
    syscw: u64,
}
impl Io {
    fn parse(raw: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(raw).ok()?;
        let mut fields = [None; 4];
        for line in text.lines() {
            let (name, value) = line.split_once(':')?;
            if let Some(index) = ["rchar", "wchar", "syscr", "syscw"]
                .iter()
                .position(|expected| *expected == name)
            {
                if fields[index]
                    .replace(value.trim().parse::<u64>().ok()?)
                    .is_some()
                {
                    return None;
                }
            }
        }
        Some(Self {
            rchar: fields[0]?,
            wchar: fields[1]?,
            syscr: fields[2]?,
            syscw: fields[3]?,
        })
    }
    fn read(file: &File) -> Self {
        let mut raw = [0; 1024];
        let count = file.read_at(&mut raw, 0).unwrap();
        assert!(count < raw.len());
        Self::parse(&raw[..count]).unwrap()
    }
    fn since(self, before: Self) -> Self {
        Self {
            rchar: self.rchar.checked_sub(before.rchar).unwrap(),
            wchar: self.wchar.checked_sub(before.wchar).unwrap(),
            syscr: self.syscr.checked_sub(before.syscr).unwrap(),
            syscw: self.syscw.checked_sub(before.syscw).unwrap(),
        }
    }
}
#[test]
fn authority_breakdown_io_parser_refuses_missing_duplicate_overflow_and_bad_text() {
    let valid = b"rchar: 1\nwchar: 2\nsyscr: 3\nsyscw: 4\nread_bytes: 0\n";
    assert_eq!(
        Io::parse(valid),
        Some(Io {
            rchar: 1,
            wchar: 2,
            syscr: 3,
            syscw: 4
        })
    );
    for raw in [
        b"".as_slice(),
        b"rchar: 1\nwchar: 2\nsyscr: 3\n",
        b"rchar: 1\nwchar: 2\nsyscr: 3\nsyscw: 4\nsyscr: 5\n",
        b"rchar: 18446744073709551616\nwchar: 2\nsyscr: 3\nsyscw: 4\n",
        b"rchar: -1\nwchar: 2\nsyscr: 3\nsyscw: 4\n",
        b"\xff",
    ] {
        assert!(Io::parse(raw).is_none());
    }
}
#[test]
fn authority_breakdown_retained_io_reader_observes_reads_without_writes() {
    let file = File::open("/proc/thread-self/io").unwrap();
    let before = Io::read(&file);
    let after = Io::read(&file).since(before);
    assert_eq!(after.syscr, 1);
    assert_eq!(after.syscw, 0);
    assert!(after.rchar > 0);
    assert_eq!(after.wchar, 0);
}

#[test]
#[ignore = "manual local request decomposition; not timing or admission"]
fn authority_breakdown_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    let io = File::open("/proc/thread-self/io").unwrap();
    println!(
        "HC074-AUTHORITY-BREAKDOWN-ORIGIN {}",
        serde_json::json!({
            "pid":std::process::id(),"boot_id":origin.boot,"uid":unsafe { libc::geteuid() },
            "iterations":1024,"samples":3,"warmups":32,"server":false,"product":false,
        })
    );
    for depth in [1, 7] {
        let mut fixture = Fixture::new(depth);
        println!("HC074-AUTHORITY-BREAKDOWN-FIXTURE {}", fixture.identity());
        for (_, phase) in PHASES {
            for _ in 0..32 {
                fixture.run(phase).unwrap();
            }
        }
        for sample in 0..3 {
            let mut counts = [Counts::default(); 9];
            let mut reads = [0; 9];
            for position in 0..PHASES.len() {
                let index = (sample + position) % PHASES.len();
                let (phase_name, phase) = PHASES[index];
                let mut run = || {
                    for _ in 0..1024 {
                        fixture.run(std::hint::black_box(phase)).unwrap();
                    }
                };
                counts[index] = capture(&mut run).1;
                let before = Io::read(&io);
                let empty = Io::read(&io).since(before);
                let before = Io::read(&io);
                run();
                let observed = Io::read(&io).since(before);
                assert_eq!(empty.syscr, 1);
                assert_eq!(empty.syscw, 0);
                assert_eq!(observed.syscw, 0);
                assert_eq!(observed.wchar, 0);
                reads[index] = observed.syscr.checked_sub(empty.syscr).unwrap();
                println!(
                    "HC074-AUTHORITY-BREAKDOWN {}",
                    serde_json::json!({
                        "generations":depth,"phase":phase_name,"sample":sample,"position":position,
                        "iterations":1024,"warmups":32,"counts":counts[index],"empty":empty,"observed":observed,
                        "allocator_active_during_io":false,"all_syscalls_observed":false,
                        "runtime_changed":false,"timing_claim":false,"rust_requests_only":true,
                        "server":false,"product":false,
                    })
                );
            }
            for metric in [
                (|c: Counts| c.allocations) as fn(Counts) -> u64,
                |c: Counts| c.reallocations,
                |c: Counts| c.requested_bytes,
            ] {
                assert_eq!(
                    metric(counts[0]),
                    2 * metric(counts[1])
                        + 2 * metric(counts[2])
                        + metric(counts[3])
                        + metric(counts[4])
                        + metric(counts[5])
                );
                assert_eq!(metric(counts[2]), metric(counts[6]) + metric(counts[7]));
                assert!(metric(counts[8]) <= metric(counts[6]));
            }
            assert_eq!(
                reads[0],
                2 * reads[1] + 2 * reads[2] + reads[3] + reads[4] + reads[5]
            );
            assert_eq!(reads[2], reads[6] + reads[7]);
        }
    }
    origin.revalidate().unwrap();
}
