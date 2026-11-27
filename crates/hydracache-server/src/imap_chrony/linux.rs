//! Linux clock watcher and off-request-path chronyc collection.

use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hydracache::{ClusterTimeAuthority, TimeObservation};
use rustix::fd::OwnedFd;
use rustix::io::Errno;
use rustix::time::{
    clock_gettime, timerfd_create, timerfd_settime, ClockId, Itimerspec, TimerfdClockId,
    TimerfdFlags, TimerfdTimerFlags, Timespec,
};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::watch;

use super::{ChronySample, ClockReading, NS_PER_SEC};
use crate::config::IMapChronyConfig;

const MAX_REPORT_BYTES: u64 = 1025;
const WATCH_SECONDS: i64 = 365 * 24 * 60 * 60;

pub(crate) struct ChronyClock {
    state: Mutex<State>,
}

impl ChronyClock {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(State::new()),
        }
    }

    pub(crate) async fn run(
        self: Arc<Self>,
        config: IMapChronyConfig,
        mut shutdown: watch::Receiver<bool>,
    ) {
        let _guard = CollectorGuard(&self);
        let mut ticks = tokio::time::interval(Duration::from_millis(config.sample_interval_ms));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        while !*shutdown.borrow() {
            tokio::select! {
                _ = shutdown.changed() => return,
                _ = ticks.tick() => {}
            }
            tokio::select! {
                _ = shutdown.changed() => return,
                _ = self.refresh(&config) => {}
            }
        }
    }

    async fn refresh(&self, config: &IMapChronyConfig) {
        let Some(before) = self.state.lock().ok().and_then(|mut state| state.read()) else {
            return;
        };
        let mut command = Command::new(&config.chronyc);
        command
            .args(["-n", "-c", "-h", "127.0.0.1", "tracking"])
            .env("LC_ALL", "C");
        let report = read_tracking(
            &mut command,
            Duration::from_millis(config.policy.maximum_acquisition_ms),
        )
        .await;
        if let Ok(mut state) = self.state.lock() {
            state.sample = report
                .zip(state.read())
                .and_then(|(report, after)| {
                    ChronySample::capture(
                        &report,
                        &config.references,
                        [before, after],
                        config.policy,
                    )
                    .ok()
                })
                .filter(|sample| sample.reference_ns > state.previous_generation_reference);
            if let Some(sample) = &state.sample {
                state.latest_reference = state.latest_reference.max(sample.reference_ns);
            }
        }
    }
}

impl ClusterTimeAuthority for ChronyClock {
    fn observe(&self) -> Option<TimeObservation> {
        let mut state = self.state.lock().ok()?;
        state.sample.as_ref()?;
        let now = state.read()?;
        state.sample.as_ref()?.observe(now).ok()
    }
}

struct CollectorGuard<'a>(&'a ChronyClock);

impl Drop for CollectorGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.sample = None;
            state.generation = None;
        }
    }
}

struct State {
    watch: Option<KernelWatcher>,
    generation: Option<u64>,
    sample: Option<ChronySample>,
    latest_reference: u64,
    previous_generation_reference: u64,
}

impl State {
    fn new() -> Self {
        Self {
            watch: None,
            generation: Some(1),
            sample: None,
            latest_reference: 0,
            previous_generation_reference: 0,
        }
    }

    fn read(&mut self) -> Option<ClockReading> {
        let generation = self.generation?;
        if self.watch.is_none() {
            self.watch = KernelWatcher::open().ok();
        }
        let reading = self.watch.as_mut()?.read(generation);
        if reading.is_none() {
            self.invalidate();
        }
        reading
    }

    fn invalidate(&mut self) {
        self.watch = None;
        self.sample = None;
        self.previous_generation_reference = self.latest_reference;
        self.generation = self
            .generation
            .and_then(|generation| generation.checked_add(1));
    }
}

struct KernelWatcher {
    timer: OwnedFd,
    offset: Option<(i128, i128)>,
}

impl KernelWatcher {
    fn open() -> Result<Self, Errno> {
        let timer = timerfd_create(
            TimerfdClockId::Realtime,
            TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
        )?;
        timerfd_settime(
            &timer,
            TimerfdTimerFlags::ABSTIME | TimerfdTimerFlags::CANCEL_ON_SET,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: Timespec {
                    tv_sec: clock_gettime(ClockId::Realtime)
                        .tv_sec
                        .checked_add(WATCH_SECONDS)
                        .ok_or(Errno::OVERFLOW)?,
                    tv_nsec: 0,
                },
            },
        )?;
        Ok(Self {
            timer,
            offset: None,
        })
    }

    fn read(&mut self, generation: u64) -> Option<ClockReading> {
        let monotonic_before = time_ns(ClockId::Monotonic)?;
        let boot_before = time_ns(ClockId::Boottime)?;
        let realtime_ns = time_ns(ClockId::Realtime)?;
        let boot_after = time_ns(ClockId::Boottime)?;
        let monotonic_after = time_ns(ClockId::Monotonic)?;
        // An unexpired timer returns EAGAIN. A step returns ECANCELED.
        // A timer expiration, a short read or any error also withdraws evidence.
        if !watch_intact(rustix::io::read(&self.timer, &mut [0_u8; 8])) {
            return None;
        }
        let offset = (
            i128::from(boot_before) - i128::from(monotonic_after),
            i128::from(boot_after) - i128::from(monotonic_before),
        );
        self.offset = Some(intersect_offset(self.offset, offset)?);
        Some(ClockReading {
            realtime_ns,
            boottime_ns: boot_after,
            read_window_ns: boot_after.checked_sub(boot_before)?,
            generation,
        })
    }
}

fn watch_intact(result: Result<usize, Errno>) -> bool {
    result == Err(Errno::AGAIN)
}

fn intersect_offset(previous: Option<(i128, i128)>, current: (i128, i128)) -> Option<(i128, i128)> {
    let previous = previous.unwrap_or(current);
    let low = current.0.max(previous.0);
    let high = current.1.min(previous.1);
    (low <= high).then_some((low, high))
}

fn time_ns(id: ClockId) -> Option<u64> {
    let time = clock_gettime(id);
    u64::try_from(time.tv_sec)
        .ok()?
        .checked_mul(NS_PER_SEC)?
        .checked_add(u64::try_from(time.tv_nsec).ok()?)
}

async fn read_tracking(command: &mut Command, timeout: Duration) -> Option<String> {
    tokio::time::timeout(timeout, async {
        let mut child = command
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut report = Vec::new();
        child
            .stdout
            .take()?
            .take(MAX_REPORT_BYTES + 1)
            .read_to_end(&mut report)
            .await
            .ok()?;
        if report.len() as u64 > MAX_REPORT_BYTES || !child.wait().await.ok()?.success() {
            return None;
        }
        String::from_utf8(report).ok()
    })
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polls_real_kernel_clock_and_fences_timer_and_suspend_signals() {
        let mut state = State::new();
        let first = state.read().unwrap();
        let second = state.read().unwrap();
        assert!(second.boottime_ns >= first.boottime_ns);
        assert_eq!(first.generation, second.generation);
        assert!(!watch_intact(Err(Errno::CANCELED)));
        assert!(!watch_intact(Ok(8)));
        assert!(!watch_intact(Err(Errno::BADF)));
        assert_eq!(intersect_offset(Some((-9, 12)), (-12, 7)), Some((-9, 7)));
        assert_eq!(intersect_offset(Some((-9, 7)), (8, 90)), None);
        state.latest_reference = 23;
        state.invalidate();
        assert_eq!(state.read().unwrap().generation, first.generation + 1);
        assert_eq!(state.previous_generation_reference, 23);
    }

    #[tokio::test]
    async fn collector_reports_success_and_rejects_exit_overflow_and_hang() {
        let timeout = Duration::from_secs(2);
        let mut echo = Command::new("/bin/echo");
        echo.arg("report");
        assert_eq!(
            read_tracking(&mut echo, timeout).await.as_deref(),
            Some("report\n")
        );
        assert!(read_tracking(&mut Command::new("/bin/false"), timeout)
            .await
            .is_none());
        assert!(read_tracking(echo.arg("x".repeat(1025)), timeout)
            .await
            .is_none());
        let mut sleep = Command::new("/bin/sleep");
        sleep.arg("30");
        assert!(read_tracking(&mut sleep, Duration::from_millis(30))
            .await
            .is_none());
    }
}
