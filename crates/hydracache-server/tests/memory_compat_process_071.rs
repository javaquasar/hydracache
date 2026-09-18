mod support;

use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};

use support::daemon_cluster::{ensure_distinct_daemon_binaries, DaemonCluster, TestResult};

const REQUIRED_ENV: &str = "HYDRACACHE_MEMORY_071_COMPAT_REQUIRED";
const BASELINE_BINARY_ENV: &str = "HYDRACACHE_MEMORY_071_BASELINE_BINARY";
const CANDIDATE_BINARY_ENV: &str = "HYDRACACHE_MEMORY_071_CANDIDATE_BINARY";

#[test]
fn real_v070_candidate_rolling_upgrade_restart_and_rollback() -> TestResult {
    let Some((baseline, candidate)) = compatibility_binaries()? else {
        return Ok(());
    };
    let mut cluster = DaemonCluster::start_bootstrap_with_binaries(
        vec![baseline.clone(); 3],
        "memory-071-real-upgrade-rollback",
    )?;
    let expected_members = cluster.node_ids().into_iter().collect::<BTreeSet<_>>();
    assert_consensus(&mut cluster, &expected_members)?;

    for index in 0..3 {
        cluster.kill(index)?;
        cluster.restart_with_binary(index, candidate.clone())?;
        assert_consensus(&mut cluster, &expected_members)?;
    }
    for index in 0..3 {
        cluster.kill(index)?;
        cluster.restart(index)?;
        assert_consensus(&mut cluster, &expected_members)?;
    }

    for index in (0..3).rev() {
        cluster.kill(index)?;
        cluster.restart_with_binary(index, baseline.clone())?;
        assert_consensus(&mut cluster, &expected_members)?;
    }
    assert!(cluster
        .binary_paths()
        .iter()
        .all(|path| path == baseline.as_path()));
    Ok(())
}

#[test]
fn real_v070_candidate_mixed_role_orders_form_the_same_cluster() -> TestResult {
    let Some((baseline, candidate)) = compatibility_binaries()? else {
        return Ok(());
    };
    for (index, binaries) in [
        vec![candidate.clone(), baseline.clone(), baseline.clone()],
        vec![baseline.clone(), candidate.clone(), baseline.clone()],
        vec![baseline.clone(), baseline.clone(), candidate.clone()],
    ]
    .into_iter()
    .enumerate()
    {
        let mut cluster = DaemonCluster::start_bootstrap_with_binaries(
            binaries,
            &format!("memory-071-mixed-order-{index}"),
        )?;
        let expected_members = cluster.node_ids().into_iter().collect::<BTreeSet<_>>();
        assert_consensus(&mut cluster, &expected_members)?;
    }
    Ok(())
}

fn compatibility_binaries() -> TestResult<Option<(PathBuf, PathBuf)>> {
    if env::var(REQUIRED_ENV).as_deref() != Ok("1") {
        eprintln!("skipping real 0.71 compatibility proof; {REQUIRED_ENV}=1 was not requested");
        return Ok(None);
    }
    let baseline = required_binary(BASELINE_BINARY_ENV)?;
    let candidate = required_binary(CANDIDATE_BINARY_ENV)?;
    ensure_distinct_daemon_binaries(&baseline, &candidate)?;
    Ok(Some((baseline, candidate)))
}

fn required_binary(name: &str) -> TestResult<PathBuf> {
    let value = env::var_os(name).ok_or_else(|| format!("{name} is required"))?;
    let path = PathBuf::from(value);
    if !Path::new(&path).is_file() {
        return Err(format!("{name} is not a file: {}", path.display()).into());
    }
    Ok(path.canonicalize()?)
}

fn assert_consensus(
    cluster: &mut DaemonCluster,
    expected_members: &BTreeSet<String>,
) -> TestResult {
    let statuses = cluster.wait_for_responsive_shape(3, 3, 3)?;
    if statuses.iter().any(|status| {
        status.members != expected_members.len() as u32
            || status.voters != expected_members.len() as u32
    }) {
        return Err(format!("mixed-version cluster did not converge: {statuses:?}").into());
    }
    Ok(())
}
