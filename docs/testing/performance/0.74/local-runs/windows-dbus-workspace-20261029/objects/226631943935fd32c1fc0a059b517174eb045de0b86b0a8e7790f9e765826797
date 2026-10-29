//! Standalone read-only Linux x86_64 clock observer, not an admission implementation.
#![deny(warnings)]
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
compile_error!("The explicit syscall ABI requires Linux x86_64");

use std::ffi::{c_int, c_long};
use std::io::{self, Write};

#[repr(C)]
#[derive(Default)]
struct Timespec {
    sec: c_long,
    nsec: c_long,
}
unsafe extern "C" {
    fn clock_gettime(id: c_int, value: *mut Timespec) -> c_int;
    fn syscall(number: c_long, ...) -> c_long;
    fn sched_getcpu() -> c_int;
}
const COUNT: usize = 601;
const PERIOD_MS: u64 = 100;
// Linux x86_64 __NR_clock_gettime; never clock_settime/clock_adjtime/adjtimex.
const SYS_CLOCK_GETTIME: c_long = 228;
const CLOCKS: [c_int; 4] = [7, 0, 1, 4]; // BOOTTIME, REALTIME, MONOTONIC, RAW

fn nanos(sec: c_long, nsec: c_long) -> io::Result<u64> {
    if sec < 0 || !(0..1_000_000_000).contains(&nsec) {
        return Err(io::Error::other("invalid timespec"));
    }
    (sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|s| s.checked_add(nsec as u64))
        .ok_or_else(|| io::Error::other("timespec overflow"))
}
fn read(id: c_int, direct: bool) -> io::Result<u64> {
    let mut value = Timespec::default();
    // SAFETY: explicit Linux x86_64 ABI and a writable native timespec.
    let rc = unsafe {
        if direct {
            syscall(SYS_CLOCK_GETTIME, id, &mut value as *mut Timespec)
        } else {
            c_long::from(clock_gettime(id, &mut value))
        }
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    nanos(value.sec, value.nsec)
}
fn main() -> io::Result<()> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(out, "{{\"schema\":\"hc074-clock-readonly-v1\",\"samples\":{COUNT},\"period_ms\":{PERIOD_MS},\"clock_ids\":[7,0,1,4],\"order\":\"clock-major libc-before,syscall,libc-after\"}}")?;
    for i in 0..COUNT {
        // SAFETY: sched_getcpu has no arguments or mutable process/global state.
        let before_cpu = unsafe { sched_getcpu() };
        let mut values = [[0; 3]; 4];
        for (slot, id) in values.iter_mut().zip(CLOCKS) {
            *slot = [read(id, false)?, read(id, true)?, read(id, false)?];
        }
        let after_cpu = unsafe { sched_getcpu() };
        if before_cpu < 0 || after_cpu < 0 {
            return Err(io::Error::last_os_error());
        }
        writeln!(
            out,
            "{{\"i\":{i},\"cpu\":[{before_cpu},{after_cpu}],\"clocks\":{values:?}}}"
        )?;
        out.flush()?;
        if i + 1 != COUNT {
            std::thread::sleep(std::time::Duration::from_millis(PERIOD_MS));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timespec_bounds_and_units() {
        assert_eq!(nanos(2, 7).unwrap(), 2_000_000_007);
        assert!(nanos(-1, 0).is_err());
        assert!(nanos(0, -1).is_err());
        assert!(nanos(0, 1_000_000_000).is_err());
        assert!(nanos(c_long::MAX, 0).is_err());
    }
}
