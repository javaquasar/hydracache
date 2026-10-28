//! Pinning the threads of a benchmark to CPUs of their own.
//!
//! Left to itself, the operating system places a benchmark's threads afresh in every process: on
//! one core or on several, and on cores near each other or far apart. The threads keep to where
//! they first ran, so every sample of a process measures the same placement, and the next process
//! measures another. Where the threads wait for each other, the placement changes how long the
//! waits take by more than the code being measured does. Pinned, each thread runs on the same CPU
//! in every process.
//!
//! Pinning is only done on Linux, which is where the benchmarks are tracked. Elsewhere, the
//! threads are left where the operating system places them.

#[cfg(target_os = "linux")]
use rustix::thread::{CpuSet, sched_getaffinity, sched_setaffinity};

/// The CPUs a thread may run on, in the order they are handed out to threads to be pinned to.
pub struct Cpus {
    /// Their ids, in ascending order.
    #[cfg(target_os = "linux")]
    ids: Vec<usize>,
}

impl Cpus {
    /// The CPUs the calling thread may run on.
    pub fn allowed() -> Self {
        #[cfg(target_os = "linux")]
        {
            let allowed = sched_getaffinity(None).expect("the CPUs this thread may run on");
            let ids = (0..CpuSet::MAX_CPU)
                .filter(|&id| allowed.is_set(id))
                .collect();

            Self { ids }
        }
        #[cfg(not(target_os = "linux"))]
        Self {}
    }

    /// The `n`th of them, counting from the first again past the last, so that more threads
    /// than CPUs still each get one, if not one of their own.
    pub fn nth(&self, n: usize) -> Cpu {
        #[cfg(target_os = "linux")]
        {
            Cpu {
                id: self.ids[n % self.ids.len()],
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = n;

            Cpu {}
        }
    }
}

/// One of [`Cpus`].
#[derive(Clone, Copy)]
pub struct Cpu {
    #[cfg(target_os = "linux")]
    id: usize,
}

impl Cpu {
    /// Pins the calling thread to this CPU, until the returned guard is dropped.
    pub fn pin(self) -> Pinned {
        #[cfg(target_os = "linux")]
        {
            let previous = sched_getaffinity(None).expect("the CPUs this thread may run on");
            let mut pinned = CpuSet::new();
            pinned.set(self.id);
            sched_setaffinity(None, &pinned).expect("pin the thread to its CPU");

            Pinned { previous }
        }
        #[cfg(not(target_os = "linux"))]
        Pinned {}
    }
}

/// A thread's pinning to a CPU, which ends when this is dropped: the thread may then run on the
/// CPUs it could before again.
#[must_use = "the thread is unpinned as soon as this is dropped"]
pub struct Pinned {
    #[cfg(target_os = "linux")]
    previous: CpuSet,
}

#[cfg(target_os = "linux")]
impl Drop for Pinned {
    fn drop(&mut self) {
        sched_setaffinity(None, &self.previous).expect("unpin the thread");
    }
}
