//! Diagnostic normal-build child: no cfg(test) allocator/counters are linked.
use super::*;
use serde_json::json;
use std::hint::black_box;

#[derive(serde::Deserialize)]
struct Table {
    seed: u64,
    cases: Vec<Case>,
}
#[derive(serde::Deserialize, serde::Serialize)]
struct Case {
    name: String,
    mtu: usize,
    requests: Vec<Request>,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
enum Request {
    #[serde(rename = "node")]
    Node { id: String, port: u16 },
    #[serde(rename = "kv")]
    Kv {
        key: String,
        value: String,
        version: u64,
    },
    #[serde(rename = "max")]
    Max { version: u64 },
}
struct Outcome {
    decisions: [bool; 16],
    attempted: usize,
    wire: Vec<u8>,
}
fn run(case: &Case) -> Outcome {
    let mut s = DeltaSerializer::with_mtu(black_box(case.mtu));
    let mut result = Outcome {
        decisions: [false; 16],
        attempted: 0,
        wire: Vec::new(),
    };
    assert!(case.requests.len() <= 16);
    for request in &case.requests {
        let accepted = match black_box(request) {
            Request::Node { id, port } => s.try_add_node(
                ChitchatId::new(id.clone(), 1, ([127, 0, 0, 1], *port).into()),
                0,
                0,
            ),
            Request::Kv {
                key,
                value,
                version,
            } => s.try_add_kv(
                key,
                VersionedValue {
                    value: value.to_string(),
                    version: *version,
                    status: crate::DeletionStatus::Set,
                },
            ),
            Request::Max { version } => s.try_set_max_version(*version),
        };
        result.decisions[result.attempted] = accepted;
        result.attempted += 1;
        if !accepted {
            break;
        }
    }
    let delta = s.finish();
    result.wire = black_box(delta.serialize_to_vec());
    drop(black_box(delta));
    result
}
fn semantics(cases: &[Case]) -> serde_json::Value {
    let mut rows = Vec::new();
    for case in cases {
        let r = run(case);
        let mut view = &r.wire[..];
        let delta = Delta::deserialize(&mut view).unwrap();
        assert!(view.is_empty());
        assert_eq!(delta.serialize_to_vec(), r.wire);
        if VARIANT != "B" {
            assert!(r.wire.len() <= case.mtu);
        }
        rows.push(json!({"input":case,"decisions":&r.decisions[..r.attempted],
            "wire":r.wire.iter().map(|b|format!("{b:02x}")).collect::<String>()}));
    }
    json!(rows)
}
#[repr(C)]
#[derive(Default)]
struct Timespec {
    sec: i64,
    nsec: i64,
}
#[repr(C)]
#[derive(Default, Clone, Copy, serde::Serialize)]
struct Heap {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}
unsafe extern "C" {
    fn clock_gettime(id: i32, out: *mut Timespec) -> i32;
    fn clock_getres(id: i32, out: *mut Timespec) -> i32;
    fn mallinfo2() -> Heap;
}
fn clock(id: i32, res: bool) -> u64 {
    let mut t = Timespec::default();
    // SAFETY: x86_64 Linux/glibc ABI, live aligned writable timespec; no ownership.
    assert_eq!(
        unsafe {
            if res {
                clock_getres(id, &mut t)
            } else {
                clock_gettime(id, &mut t)
            }
        },
        0
    );
    assert!(t.sec >= 0 && (0..1_000_000_000).contains(&t.nsec));
    (t.sec as u64)
        .checked_mul(1_000_000_000)
        .unwrap()
        .checked_add(t.nsec as u64)
        .unwrap()
}
#[derive(Clone, Copy, Default, serde::Serialize)]
struct Stamp {
    raw: u64,
    mono: u64,
    boot: u64,
    cpu: u64,
}
impl Stamp {
    fn now() -> Self {
        Self {
            raw: clock(4, false),
            mono: clock(1, false),
            boot: clock(7, false),
            cpu: clock(3, false),
        }
    }
    fn since(self, b: Self) -> Self {
        Self {
            raw: self.raw.checked_sub(b.raw).unwrap(),
            mono: self.mono.checked_sub(b.mono).unwrap(),
            boot: self.boot.checked_sub(b.boot).unwrap(),
            cpu: self.cpu.checked_sub(b.cpu).unwrap(),
        }
    }
}
#[derive(Clone, Copy, Default, serde::Serialize)]
struct Memory {
    heap: Heap,
    rss: u64,
    pss: u64,
    private_clean: u64,
    private_dirty: u64,
    threads: u64,
}
fn proc_read(path: &str, buf: &mut [u8]) -> usize {
    use std::io::Read;
    let mut file = std::fs::File::open(path).unwrap();
    let mut n = 0;
    loop {
        let got = file.read(&mut buf[n..]).unwrap();
        if got == 0 {
            break;
        }
        n += got;
        assert!(n < buf.len(), "complete fixed proc buffer");
    }
    n
}
fn field(text: &str, key: &str) -> u64 {
    let mut values = text.lines().filter_map(|s| s.strip_prefix(key));
    let raw = values.next().unwrap();
    assert!(values.next().is_none());
    raw.split_whitespace().next().unwrap().parse().unwrap()
}
fn memory() -> Memory {
    let mut buf = [0u8; 8192];
    let n = proc_read("/proc/self/smaps_rollup", &mut buf);
    let text = std::str::from_utf8(&buf[..n]).unwrap();
    let mut value = Memory {
        rss: field(text, "Rss:") * 1024,
        pss: field(text, "Pss:") * 1024,
        private_clean: field(text, "Private_Clean:") * 1024,
        private_dirty: field(text, "Private_Dirty:") * 1024,
        ..Memory::default()
    };
    let n = proc_read("/proc/self/status", &mut buf);
    value.threads = field(std::str::from_utf8(&buf[..n]).unwrap(), "Threads:");
    // SAFETY: glibc >=2.33 exact ten-size_t return ABI, current single-thread probe.
    value.heap = unsafe { mallinfo2() };
    assert_eq!(value.threads, 1);
    value
}
fn sweep(cases: &[Case]) {
    for case in cases {
        drop(black_box(run(black_box(case))));
    }
}
pub fn main() {
    assert_eq!(std::mem::size_of::<usize>(), 8);
    assert_eq!(std::mem::size_of::<Heap>(), 80);
    let mode = std::env::args().nth(1).unwrap();
    assert!(matches!(mode.as_str(), "control" | "cpu" | "memory"));
    let table: Table = serde_json::from_str(include_str!("cases-r2.json")).unwrap();
    assert_eq!(table.seed, 0x0741028);
    assert_eq!(table.cases.len(), 30);
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap();
    let pid = std::process::id();
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let start_ticks: u64 = stat
        .rsplit_once(") ")
        .unwrap()
        .1
        .split_whitespace()
        .nth(19)
        .unwrap()
        .parse()
        .unwrap();
    let before = semantics(&table.cases);
    let resolutions = [
        clock(4, true),
        clock(1, true),
        clock(7, true),
        clock(3, true),
    ];
    for _ in 0..4 {
        sweep(&table.cases);
    }
    let overall = Stamp::now();
    let mut timings = Vec::with_capacity(30);
    let mut snapshots = [Memory::default(); 7];
    match mode.as_str() {
        "cpu" => {
            for case in &table.cases {
                let at = Stamp::now();
                for _ in 0..32 {
                    drop(black_box(run(black_box(case))));
                }
                timings.push(Stamp::now().since(at));
            }
        }
        "memory" => {
            snapshots[0] = memory();
            for slot in snapshots.iter_mut().skip(1) {
                for _ in 0..8 {
                    sweep(&table.cases);
                }
                *slot = memory();
            }
        }
        "control" => {
            snapshots[0] = memory();
            sweep(&table.cases);
            snapshots[1] = memory();
        }
        _ => unreachable!(),
    }
    let elapsed = Stamp::now().since(overall);
    assert_eq!(before, semantics(&table.cases));
    assert_eq!(
        boot,
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id").unwrap()
    );
    assert_eq!(
        start_ticks,
        std::fs::read_to_string("/proc/self/stat")
            .unwrap()
            .rsplit_once(") ")
            .unwrap()
            .1
            .split_whitespace()
            .nth(19)
            .unwrap()
            .parse::<u64>()
            .unwrap()
    );
    println!(
        "HC074_CLEAN_COST:{}",
        json!({"variant":VARIANT,"mode":mode,"seed":table.seed,"rows":before,"pid":pid,"start_ticks":start_ticks,
        "boot_id":boot.trim(),"resolutions_ns":resolutions,"timings":timings,"memory":snapshots,"elapsed":elapsed,
        "test_cfg":false,"allocation_wrapper":false,"cpu_repetitions":32,"memory_sweeps":8,"memory_epochs":6,"warm_sweeps":4})
    );
}
