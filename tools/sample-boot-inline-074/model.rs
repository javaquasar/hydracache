//! Whole copied point/sample/decoder bodies; Origin and receivers are synthetic.
use crate::{clock, libc, KernelError};

// BEGIN POINT CORE
#[derive(Debug, Clone)]
pub struct KernelTimePoint {
    boot: String,
    boottime: u64,
    pub(super) realtime_ns: u64,
}
pub(super) type TimePoint = KernelTimePoint;
impl KernelTimePoint {
    pub fn boot_id(&self) -> &str {
        &self.boot
    }
    pub fn boottime_ns(&self) -> u64 {
        self.boottime
    }
    pub fn unix_seconds(&self) -> u64 {
        self.realtime_ns / 1_000_000_000
    }
}
// END POINT CORE

pub(super) struct Origin {
    boot: String,
}
impl Origin {
    // BEGIN SAMPLE CORE
    pub(super) fn sample(&self) -> Result<KernelTimePoint, KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sample,
        );
        self.revalidate()?;
        let boottime = clock(libc::CLOCK_BOOTTIME)?;
        let realtime_ns = clock(libc::CLOCK_REALTIME)?;
        self.revalidate()?;
        Ok(KernelTimePoint {
            boot: self.boot.clone(),
            boottime,
            realtime_ns,
        })
    }
    // END SAMPLE CORE

    // Separate stage control, not substituted into sample.
    fn stage_prelude(&self) -> Result<(u64, u64), KernelError> {
        #[cfg(test)]
        let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
            crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Sample,
        );
        self.revalidate()?;
        let boottime = clock(libc::CLOCK_BOOTTIME)?;
        let realtime_ns = clock(libc::CLOCK_REALTIME)?;
        self.revalidate()?;
        Ok((boottime, realtime_ns))
    }
}

// BEGIN DECODER CORE
fn boot_value(raw: ([u8; 38], usize)) -> Result<String, KernelError> {
    boot_view(&raw).map(str::to_owned)
}
fn boot_view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {
    let (raw, count) = raw;
    let count = *count;
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    std::str::from_utf8(&raw[..36]).map_err(|_| kernel_invalid!("boot-utf8"))
}
// END DECODER CORE

#[path = "../sample-boot-baseline-074/controls.rs"]
mod controls;

#[path = "cost.rs"]
mod cost;
