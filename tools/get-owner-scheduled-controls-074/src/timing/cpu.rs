//! Whole current-process CPU only, not server CPU or a sampling profiler.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Clock {
    pub provider: &'static str,
    pub scope: &'static str,
    /// API representation/getres unit. NOT measured accuracy/update frequency.
    pub unit_resolution_ns: u64,
    pub resolution_is_accuracy_claim: bool,
}
pub fn delta(before: u64, after: u64) -> Result<u64, String> {
    after
        .checked_sub(before)
        .ok_or_else(|| "process CPU clock moved backwards".to_owned())
}

#[cfg(windows)]
pub fn read() -> Result<(u64, Clock), String> {
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessTimes(
            process: *mut std::ffi::c_void,
            created: *mut FileTime,
            exited: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    let mut created = FileTime::default();
    let mut exited = FileTime::default();
    let mut kernel = FileTime::default();
    let mut user = FileTime::default();
    // SAFETY: initialized repr(C) FILETIMEs, writable and disjoint, and the
    // current-process pseudo-handle is never closed or shared with another PID.
    if unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(format!(
            "GetProcessTimes: {}",
            std::io::Error::last_os_error()
        ));
    }
    let ticks = |t: FileTime| (u64::from(t.high) << 32) | u64::from(t.low);
    let ns = ticks(kernel)
        .checked_add(ticks(user))
        .and_then(|t| t.checked_mul(100))
        .ok_or("process CPU time overflow")?;
    Ok((
        ns,
        Clock {
            provider: "GetProcessTimes",
            scope: "whole-process-user-plus-kernel",
            unit_resolution_ns: 100,
            resolution_is_accuracy_claim: false,
        },
    ))
}

#[cfg(target_os = "linux")]
pub fn read() -> Result<(u64, Clock), String> {
    #[repr(C)]
    #[derive(Default)]
    struct Timespec {
        seconds: std::ffi::c_long,
        nanoseconds: std::ffi::c_long,
    }
    unsafe extern "C" {
        fn clock_gettime(id: std::ffi::c_int, time: *mut Timespec) -> std::ffi::c_int;
        fn clock_getres(id: std::ffi::c_int, time: *mut Timespec) -> std::ffi::c_int;
    }
    const PROCESS_CPU: std::ffi::c_int = 2;
    let mut time = Timespec::default();
    let mut resolution = Timespec::default();
    // SAFETY: Linux CLOCK_PROCESS_CPUTIME_ID and initialized native-long
    // timespec layouts; both outputs point to live writable storage.
    if unsafe { clock_gettime(PROCESS_CPU, &mut time) } != 0
        || unsafe { clock_getres(PROCESS_CPU, &mut resolution) } != 0
    {
        return Err(format!(
            "process CPU clock: {}",
            std::io::Error::last_os_error()
        ));
    }
    let ns = |t: Timespec| -> Result<u64, String> {
        if t.seconds < 0 || !(0..1_000_000_000).contains(&t.nanoseconds) {
            return Err("invalid process CPU timespec".to_owned());
        }
        (t.seconds as u64)
            .checked_mul(1_000_000_000)
            .and_then(|s| s.checked_add(t.nanoseconds as u64))
            .ok_or("process CPU overflow".to_owned())
    };
    Ok((
        ns(time)?,
        Clock {
            provider: "CLOCK_PROCESS_CPUTIME_ID",
            scope: "whole-process-all-threads",
            unit_resolution_ns: ns(resolution)?.max(1),
            resolution_is_accuracy_claim: false,
        },
    ))
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn read() -> Result<(u64, Clock), String> {
    Err("whole-process CPU provider unsupported on this platform".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delta_refuses_backwards_clock_and_does_not_fabricate_nonzero_time() {
        assert_eq!(delta(10, 10).unwrap(), 0);
        assert_eq!(delta(10, 13).unwrap(), 3);
        assert!(delta(13, 10).is_err());
    }
    #[test]
    fn current_process_clock_has_units_without_an_accuracy_claim() {
        let (before, clock) = read().unwrap();
        let (after, _) = read().unwrap();
        delta(before, after).unwrap();
        assert!(clock.unit_resolution_ns > 0);
        assert!(!clock.resolution_is_accuracy_claim);
    }
}
