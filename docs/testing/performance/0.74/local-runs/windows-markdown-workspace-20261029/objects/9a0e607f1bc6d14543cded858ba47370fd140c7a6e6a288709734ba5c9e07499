//! Local actual-phase attribution only; never a replacement validation schedule.
use super::*;
use crate::diagnostic_worker_policy::authority_registry::fixture::owners_controls::{
    Fixture, Observer, Phase, Site, Uncounted, PHASES, SITES,
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
fn authority_owners_io_parser_refuses_missing_duplicate_overflow_and_bad_text() {
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
fn authority_owners_retained_io_reader_observes_reads_without_writes() {
    let file = File::open("/proc/thread-self/io").unwrap();
    let before = Io::read(&file);
    let after = Io::read(&file).since(before);
    assert_eq!(after.syscr, 1);
    assert_eq!(after.syscw, 0);
    assert!(after.rchar > 0);
    assert_eq!(after.wchar, 0);
}

#[derive(Default)]
struct Counting {
    counts: [Counts; 13],
    calls: [u64; 13],
}
fn add(left: &mut Counts, right: Counts) {
    left.allocations = left.allocations.checked_add(right.allocations).unwrap();
    left.reallocations = left.reallocations.checked_add(right.reallocations).unwrap();
    left.requested_bytes = left
        .requested_bytes
        .checked_add(right.requested_bytes)
        .unwrap();
}
impl Counting {
    fn total(&self) -> Counts {
        let mut total = Counts::default();
        for count in self.counts {
            add(&mut total, count);
        }
        total
    }
}
impl Observer for Counting {
    fn around<T>(&mut self, site: Site, call: impl FnOnce() -> T) -> T {
        assert!(
            !COUNTS.with(|state| state.get().active),
            "no nested whole-operation capture"
        );
        let (value, count) = capture(call);
        let index = site as usize;
        add(&mut self.counts[index], count);
        self.calls[index] = self.calls[index].checked_add(1).unwrap();
        value
    }
}
#[test]
fn authority_owners_expression_counter_retains_values_and_partitions_requests() {
    let mut observer = Counting::default();
    let value = observer.around(Site::Buffer, || std::hint::black_box(vec![7_u8; 64]));
    assert_eq!(value, vec![7; 64]);
    assert_eq!(
        observer.counts[Site::Buffer as usize],
        Counts {
            allocations: 1,
            reallocations: 0,
            requested_bytes: 64
        }
    );
    assert_eq!(observer.around(Site::Path, || 42), 42);
    assert_eq!(observer.calls[Site::Buffer as usize], 1);
    assert_eq!(observer.calls[Site::Path as usize], 1);
    assert_eq!(observer.total(), observer.counts[Site::Buffer as usize]);
    assert!(!COUNTS.with(|state| state.get().active));
}
#[test]
#[ignore = "manual local expression allocation owners; not timing or admission"]
fn authority_owners_allocation_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    let io = File::open("/proc/thread-self/io").unwrap();
    println!(
        "HC074-AUTHORITY-OWNERS-ORIGIN {}",
        serde_json::json!({
            "pid":std::process::id(),"boot_id":origin.boot,"uid":unsafe { libc::geteuid() },
            "iterations":1024,"samples":3,"warmups":32,"server":false,"product":false,
        })
    );
    for depth in [1, 7] {
        let mut fixture = Fixture::new(depth);
        println!("HC074-AUTHORITY-OWNERS-FIXTURE {}", fixture.identity());
        for (_, phase) in PHASES {
            for _ in 0..32 {
                fixture.run(phase, &mut Uncounted).unwrap();
            }
        }
        for sample in 0..3 {
            let mut counts = [Counts::default(); 9];
            let mut reads = [0_u64; 9];
            for position in 0..9 {
                let index = (sample + position) % 9;
                let (phase_name, phase) = PHASES[index];
                let mut observer = Counting::default();
                if index < 6 {
                    counts[index] = capture(|| {
                        for _ in 0..1024 {
                            fixture
                                .run(std::hint::black_box(phase), &mut Uncounted)
                                .unwrap();
                        }
                    })
                    .1;
                } else {
                    for _ in 0..1024 {
                        fixture
                            .run(std::hint::black_box(phase), &mut observer)
                            .unwrap();
                    }
                    counts[index] = observer.total();
                    let mut expected = [0_u64; 13];
                    if matches!(phase, Phase::TracedInventory) {
                        expected[..6].copy_from_slice(&[
                            1024,
                            1024,
                            (2 * depth as u64 + 1) * 1024,
                            2 * depth as u64 * 1024,
                            2 * depth as u64 * 1024,
                            1024,
                        ]);
                    } else {
                        expected[6..].fill(depth as u64 * 1024);
                    }
                    assert_eq!(observer.calls, expected);
                }
                let mut run = || {
                    for _ in 0..1024 {
                        fixture
                            .run(std::hint::black_box(phase), &mut Uncounted)
                            .unwrap();
                    }
                };
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
                    "HC074-AUTHORITY-OWNERS {}",
                    serde_json::json!({
                        "generations":depth,"phase":phase_name,"sample":sample,"position":position,
                        "iterations":1024,"warmups":32,"counts":counts[index],"empty":empty,"observed":observed,
                        "sites":SITES.iter().enumerate().map(|(site,name)|serde_json::json!({
                            "site":name,"counts":observer.counts[site],"calls":observer.calls[site],
                        })).collect::<Vec<_>>(),
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
            }
            assert_eq!(
                reads[0],
                2 * reads[1] + 2 * reads[2] + reads[3] + reads[4] + reads[5]
            );
            for (actual, traced) in [(2, 6), (3, 7), (4, 8)] {
                assert_eq!(counts[actual], counts[traced]);
                assert_eq!(reads[actual], reads[traced]);
            }
        }
    }
    origin.revalidate().unwrap();
}
