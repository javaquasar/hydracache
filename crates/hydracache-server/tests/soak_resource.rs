use hydracache_server::{ServerConfig, ServerRuntime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessResourceSample {
    rss_kib: u64,
    open_fds: u64,
}

impl ProcessResourceSample {
    fn current() -> Option<Self> {
        Some(Self {
            rss_kib: current_rss_kib()?,
            open_fds: current_open_fds()?,
        })
    }
}

#[test]
#[ignore = "manual/nightly resource plateau check"]
fn real_server_rss_and_fds_plateau_under_sustained_load() {
    let mut runtime = ServerRuntime::new(ServerConfig::default())
        .expect("default server config is valid")
        .start();
    assert!(runtime.can_serve());

    let Some(baseline) = ProcessResourceSample::current() else {
        eprintln!("skipping resource sampler: platform does not expose RSS and fd counts");
        return;
    };

    for _ in 0..1_000 {
        assert!(runtime.begin_request());
        runtime.finish_request();
    }

    let Some(after) = ProcessResourceSample::current() else {
        eprintln!("skipping resource sampler: platform stopped exposing RSS and fd counts");
        return;
    };

    assert!(
        after.rss_kib <= baseline.rss_kib.saturating_add(16 * 1024),
        "RSS grew from {} KiB to {} KiB",
        baseline.rss_kib,
        after.rss_kib
    );
    assert!(
        after.open_fds <= baseline.open_fds.saturating_add(32),
        "open fd count grew from {} to {}",
        baseline.open_fds,
        after.open_fds
    );
}

#[cfg(target_os = "linux")]
fn current_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmRSS:")?;
        value.split_whitespace().next()?.parse().ok()
    })
}

#[cfg(not(target_os = "linux"))]
fn current_rss_kib() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn current_open_fds() -> Option<u64> {
    Some(std::fs::read_dir("/proc/self/fd").ok()?.count() as u64)
}

#[cfg(not(target_os = "linux"))]
fn current_open_fds() -> Option<u64> {
    None
}
