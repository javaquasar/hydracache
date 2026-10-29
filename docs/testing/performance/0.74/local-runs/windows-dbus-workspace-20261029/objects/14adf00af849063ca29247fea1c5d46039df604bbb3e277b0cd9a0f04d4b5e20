//! Rejected-or-retained test-only representation, never a runtime admission.
use crate::{clock, libc, KernelError};

#[derive(Clone)]
enum OwnedBoot {
    Inline { bytes: [u8; 36], length: u8 },
    Heap(String),
}
impl OwnedBoot {
    fn copy_from(value: &str) -> Self {
        if value.len() <= 36 {
            let mut bytes = [0; 36];
            bytes[..value.len()].copy_from_slice(value.as_bytes());
            Self::Inline {
                bytes,
                length: value.len() as u8,
            }
        } else {
            Self::Heap(value.to_owned())
        }
    }
    fn as_str(&self) -> &str {
        match self {
            Self::Inline { bytes, length } => {
                std::str::from_utf8(&bytes[..usize::from(*length)]).expect("copied UTF-8 invariant")
            }
            Self::Heap(value) => value.as_str(),
        }
    }
}
impl std::fmt::Debug for OwnedBoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

// BEGIN POINT CORE
#[derive(Debug, Clone)]
pub struct KernelTimePoint {
    boot: OwnedBoot,
    boottime: u64,
    pub(super) realtime_ns: u64,
}
pub(super) type TimePoint = KernelTimePoint;
impl KernelTimePoint {
    pub fn boot_id(&self) -> &str {
        self.boot.as_str()
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
    pub(super) boot: String,
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
            boot: OwnedBoot::copy_from(&self.boot),
            boottime,
            realtime_ns,
        })
    }
    // END SAMPLE CORE
}
impl KernelTimePoint {
    // Model-only inspection/mutation helpers, outside the restored point core.
    pub(crate) fn heap_capacity(&self) -> usize {
        match &self.boot {
            OwnedBoot::Inline { .. } => 0,
            OwnedBoot::Heap(value) => value.capacity(),
        }
    }
    pub(crate) fn replace_boot(&mut self, value: &str) {
        self.boot = OwnedBoot::copy_from(value);
    }
}
