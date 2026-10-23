//! Pure byte-parser control. No proc read, admission, process launch or guard clock.
use std::hint::black_box;
use std::time::Instant;

#[allow(non_camel_case_types)]
mod libc {
    #[derive(Clone, Copy)]
    pub struct ucred {
        pub pid: i32,
        pub uid: u32,
        pub gid: u32,
    }
}
#[derive(Debug, PartialEq, Eq)]
enum KernelError {
    Invalid(&'static str),
}
macro_rules! kernel_invalid {
    ($site:literal) => {
        KernelError::Invalid($site)
    };
}
macro_rules! kernel_required {
    ($value:expr, $site:literal) => {
        ($value).ok_or_else(|| kernel_invalid!($site))
    };
}
#[path = "projection_groups.rs"]
mod projection_groups;
// The build helper copies the exact runtime struct/function here. cfg(test)
// observers are absent, as in an ordinary runtime image.
include!("projection.rs");

const SHAPES: [usize; 3] = [512, 2048, 8192];
const ITERATIONS: usize = 20_000;
const SEED: u64 = 0x0740_5ca1;

fn dataset(shape: usize) -> Vec<Vec<u8>> {
    (0..8)
        .map(|case| {
            let mut lines = vec![
                "Pid:\t4242\n".to_owned(),
                "Tgid:\t4242\n".to_owned(),
                "Uid:\t1000\t1000\t1000\t1000\n".to_owned(),
                "Gid:\t1001\t1001\t1001\t1001\n".to_owned(),
                "Groups:\t4 27 1001\n".to_owned(),
            ];
            while lines.iter().map(String::len).sum::<usize>() < shape {
                lines.push(format!("Noise{}:\t{}\n", lines.len(), "x".repeat(48)));
            }
            let mut seed = SEED ^ (case + shape) as u64;
            for slot in (1..lines.len()).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                lines.swap(slot, seed as usize % (slot + 1));
            }
            lines.concat().into_bytes()
        })
        .collect()
}
fn credentials() -> libc::ucred {
    libc::ucred {
        pid: 4242,
        uid: 1000,
        gid: 1001,
    }
}
fn fingerprint(inputs: &[Vec<u8>]) -> u64 {
    // A workload witness, not a cryptographic identity or authority input.
    inputs.iter().fold(0xcbf2_9ce4_8422_2325, |hash, input| {
        input.iter().fold(hash, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        })
    })
}
#[repr(C)]
struct Timespec {
    seconds: i64,
    nanos: i64,
}
#[link(name = "c")]
unsafe extern "C" {
    fn clock_gettime(id: i32, value: *mut Timespec) -> i32;
}
fn cpu() -> u64 {
    let mut value = Timespec {
        seconds: 0,
        nanos: 0,
    };
    // Linux x86_64 CLOCK_THREAD_CPUTIME_ID, diagnostic only.
    assert_eq!(unsafe { clock_gettime(3, &mut value) }, 0);
    assert!(value.seconds >= 0 && (0..1_000_000_000).contains(&value.nanos));
    (value.seconds as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(value.nanos as u64))
        .unwrap()
}
fn check() {
    for shape in SHAPES {
        let inputs = dataset(shape);
        println!("workload shape={shape} witness={:016x}", fingerprint(&inputs));
        for (case, bytes) in inputs.iter().enumerate() {
            println!("{shape}:{case}:valid:{:?}", projection(bytes, &credentials()));
            for prefix in ["Pid:", "Tgid:", "Uid:", "Gid:", "Groups:"] {
                let text = std::str::from_utf8(bytes).unwrap();
                let mut duplicate = bytes.clone();
                let line = text.lines().find(|line| line.starts_with(prefix)).unwrap();
                duplicate.extend_from_slice(line.as_bytes());
                duplicate.push(b'\n');
                println!("{shape}:{case}:duplicate:{prefix}:{:?}", projection(&duplicate, &credentials()));
                let missing = text.lines().filter(|line| !line.starts_with(prefix)).collect::<Vec<_>>().join("\n");
                println!("{shape}:{case}:missing:{prefix}:{:?}", projection(missing.as_bytes(), &credentials()));
            }
        }
    }
}
fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("check") => check(),
        Some("measure") => {
            for shape in SHAPES {
                let inputs = dataset(shape);
                let credentials = credentials();
                let wall = Instant::now();
                let before = cpu();
                for index in 0..ITERATIONS {
                    let result = projection(black_box(&inputs[index % inputs.len()]), black_box(&credentials)).unwrap();
                    assert_eq!(black_box(result).groups, [4, 27, 1001]);
                }
                let cpu_ns = cpu().checked_sub(before).unwrap();
                let wall_ns = wall.elapsed().as_nanos();
                println!("{{\"shape\":{shape},\"iterations\":{ITERATIONS},\"cpu_ns\":{cpu_ns},\"wall_ns\":{wall_ns},\"workload_witness\":\"{:016x}\",\"seed\":{SEED},\"admission\":false,\"product\":false}}", fingerprint(&inputs));
            }
        }
        _ => panic!("choose check or measure; no implicit measurement"),
    }
}
