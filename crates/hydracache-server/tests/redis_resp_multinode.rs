mod support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use support::daemon_cluster::{skip_unless_redis_resp_multinode_e2e, DaemonCluster, TestResult};

const LOCK_RELEASE_SCRIPT: &str =
    "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end";
const LOCK_EXTEND_SCRIPT: &str =
    "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('pexpire', KEYS[1], ARGV[2]) else return 0 end";

#[test]
fn canary_multinode_sentinel_cannot_be_marked_green_without_execution() {
    let sentinel_executed = std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() != Ok("W3");
    assert!(
        sentinel_executed,
        "HC-CANARY-RED:W3 deployment sentinel was marked green without execution"
    );
}

#[test]
fn multinode_resp_facade_roundtrip_survives_node_restart_or_drain() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e(
        "multinode_resp_facade_roundtrip_survives_node_restart_or_drain",
    ) {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap_with_redis(3, "redis-resp-multinode")?;
    let statuses = cluster.wait_for_shape(3, 3)?;
    let leader = statuses[0].leader.clone().expect("leader before drain");
    let redis_node = 0;
    let drain_index = cluster
        .node_ids()
        .iter()
        .enumerate()
        .find_map(|(index, node_id)| (index != redis_node && node_id != &leader).then_some(index))
        .unwrap_or(1);
    let redis_addr = cluster
        .redis_addr(redis_node)
        .expect("Redis address is enabled for this daemon cluster");

    assert_eq!(
        resp_roundtrip(redis_addr, "before", "v1")?,
        b"+OK\r\n$2\r\nv1\r\n+OK\r\n"
    );

    let _ = cluster.drain(drain_index)?;
    cluster.wait_for_non_draining_shape(
        "drain removal committed before Redis facade follower kill",
        2,
        2,
    )?;
    cluster.kill(drain_index)?;
    cluster.wait_for_shape(2, 2)?;

    assert_eq!(
        resp_roundtrip(redis_addr, "after", "v2")?,
        b"+OK\r\n$2\r\nv2\r\n+OK\r\n"
    );
    Ok(())
}

#[test]
fn multinode_resp_facade_documents_node_local_state() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("multinode_resp_facade_documents_node_local_state") {
        return Ok(());
    }

    // Flip this sentinel when the distributed RESP backend lands.

    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let write_addr = cluster
        .redis_addr(0)
        .expect("Redis address is enabled for node 0");
    let read_addr = cluster
        .redis_addr(1)
        .expect("Redis address is enabled for node 1");

    assert_eq!(
        resp_set(write_addr, "node-local:k", "visible-only-on-a")?,
        b"+OK\r\n+OK\r\n"
    );
    assert_eq!(resp_get(read_addr, "node-local:k")?, b"$-1\r\n+OK\r\n");
    assert_eq!(
        resp_get(write_addr, "node-local:k")?,
        b"$17\r\nvisible-only-on-a\r\n+OK\r\n"
    );
    Ok(())
}

#[test]
fn multinode_resp_lock_subset_is_single_endpoint_only() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("multinode_resp_lock_subset_is_single_endpoint_only") {
        return Ok(());
    }

    // Flip this sentinel when the distributed RESP backend lands.

    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-lock-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster
        .redis_addr(0)
        .expect("Redis address is enabled for node 0");
    let second_addr = cluster
        .redis_addr(1)
        .expect("Redis address is enabled for node 1");

    assert_eq!(
        resp_set_nx_px(first_addr, "node-local:lock", "owner-a", 30_000)?,
        b"+OK\r\n+OK\r\n"
    );
    assert_eq!(
        resp_set_nx_px(second_addr, "node-local:lock", "owner-b", 30_000)?,
        b"+OK\r\n+OK\r\n",
        "0.63 documents node-local Redis locks; a second endpoint has an independent lock state"
    );
    assert_eq!(
        resp_get(first_addr, "node-local:lock")?,
        b"$7\r\nowner-a\r\n+OK\r\n"
    );
    assert_eq!(
        resp_get(second_addr, "node-local:lock")?,
        b"$7\r\nowner-b\r\n+OK\r\n"
    );
    Ok(())
}

#[test]
fn cross_node_mget_del_exists_are_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_mget_del_exists_are_node_local") {
        return Ok(());
    }

    // Flip this sentinel when the distributed RESP backend lands.
    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-batch-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let key = "node-local:batch";

    assert_eq!(resp_set(first_addr, key, "owner-a")?, b"+OK\r\n+OK\r\n");
    assert_eq!(
        resp_pipeline(
            second_addr,
            &[
                vec!["MGET", key, "node-local:missing"],
                vec!["EXISTS", key],
                vec!["DEL", key],
                vec!["QUIT"],
            ],
        )?,
        b"*2\r\n$-1\r\n$-1\r\n:0\r\n:0\r\n+OK\r\n"
    );
    assert_eq!(
        resp_get(first_addr, key)?,
        b"$7\r\nowner-a\r\n+OK\r\n",
        "DEL on node B must not mutate the value owned by node A"
    );
    Ok(())
}

#[test]
fn cross_node_lock_release_is_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_lock_release_is_node_local") {
        return Ok(());
    }

    // Flip this sentinel when the distributed RESP backend lands.
    let mut cluster =
        DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-release-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let key = "node-local:release-lock";

    assert_eq!(
        resp_set_nx_px(first_addr, key, "owner-a", 30_000)?,
        b"+OK\r\n+OK\r\n"
    );
    assert_eq!(
        resp_pipeline(
            second_addr,
            &[
                vec!["EVAL", LOCK_RELEASE_SCRIPT, "1", key, "owner-a"],
                vec!["QUIT"],
            ],
        )?,
        b":0\r\n+OK\r\n"
    );
    assert_eq!(
        resp_pipeline(
            first_addr,
            &[
                vec!["SET", key, "contender", "NX", "PX", "30000"],
                vec!["GET", key],
                vec!["QUIT"],
            ],
        )?,
        b"$-1\r\n$7\r\nowner-a\r\n+OK\r\n",
        "release on node B must neither free nor replace node A's lock"
    );
    Ok(())
}

#[test]
fn cross_node_lock_extend_is_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_lock_extend_is_node_local") {
        return Ok(());
    }

    // Flip this sentinel when the distributed RESP backend lands.
    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-extend-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let key = "node-local:extend-lock";

    assert_eq!(
        resp_set_nx_px(first_addr, key, "owner-a", 30_000)?,
        b"+OK\r\n+OK\r\n"
    );
    let ttl_before = resp_pttl(first_addr, key)?;
    assert!((1..=30_000).contains(&ttl_before));
    assert_eq!(
        resp_pipeline(
            second_addr,
            &[
                vec!["EVAL", LOCK_EXTEND_SCRIPT, "1", key, "owner-a", "120000",],
                vec!["QUIT"],
            ],
        )?,
        b":0\r\n+OK\r\n"
    );
    let ttl_after = resp_pttl(first_addr, key)?;
    assert!(
        (1..=ttl_before).contains(&ttl_after),
        "node B extension must not increase node A PTTL: before={ttl_before}, after={ttl_after}"
    );
    assert_eq!(resp_get(first_addr, key)?, b"$7\r\nowner-a\r\n+OK\r\n");
    Ok(())
}

#[test]
fn cross_node_mset_is_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_mset_is_node_local") {
        return Ok(());
    }

    // Flip this sentinel when the distributed RESP backend lands.
    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-mset-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let first_key = "node-local:mset-a";
    let second_key = "node-local:mset-b";

    assert_eq!(
        resp_pipeline(
            first_addr,
            &[
                vec!["MSET", first_key, "value-a", second_key, "value-b"],
                vec!["QUIT"],
            ],
        )?,
        b"+OK\r\n+OK\r\n"
    );
    assert_eq!(
        resp_pipeline(
            second_addr,
            &[vec!["MGET", first_key, second_key], vec!["QUIT"]],
        )?,
        b"*2\r\n$-1\r\n$-1\r\n+OK\r\n"
    );
    assert_eq!(
        resp_pipeline(
            first_addr,
            &[vec!["MGET", first_key, second_key], vec!["QUIT"]],
        )?,
        b"*2\r\n$7\r\nvalue-a\r\n$7\r\nvalue-b\r\n+OK\r\n",
        "both MSET values must remain present on node A"
    );
    Ok(())
}

#[test]
fn cross_node_ttl_visibility_is_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_ttl_visibility_is_node_local") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-ttl-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let key = "node-local:ttl";

    assert_eq!(
        resp_pipeline(
            first_addr,
            &[vec!["SET", key, "value", "PX", "30000"], vec!["QUIT"]],
        )?,
        b"+OK\r\n+OK\r\n"
    );
    assert_eq!(resp_pttl(second_addr, key)?, -2);
    assert!((1..=30_000).contains(&resp_pttl(first_addr, key)?));
    Ok(())
}

#[test]
fn cross_node_script_cache_is_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_script_cache_is_node_local") {
        return Ok(());
    }

    let mut cluster =
        DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-script-cache-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let loaded = resp_pipeline(
        first_addr,
        &[vec!["SCRIPT", "LOAD", LOCK_RELEASE_SCRIPT], vec!["QUIT"]],
    )?;
    let loaded_text = std::str::from_utf8(&loaded)?;
    let sha = loaded_text
        .split("\r\n")
        .nth(1)
        .ok_or("SCRIPT LOAD did not return a SHA")?;
    let on_b = resp_pipeline(second_addr, &[vec!["SCRIPT", "EXISTS", sha], vec!["QUIT"]])?;
    let on_a = resp_pipeline(first_addr, &[vec!["SCRIPT", "EXISTS", sha], vec!["QUIT"]])?;
    assert!(
        on_b.starts_with(b"*1\r\n:0\r\n"),
        "SCRIPT cache on node B unexpectedly saw node A's script: {on_b:?}"
    );
    assert!(
        on_a.starts_with(b"*1\r\n:1\r\n"),
        "SCRIPT cache on node A did not retain loaded script: {on_a:?}"
    );
    Ok(())
}

#[test]
fn cross_node_tag_index_is_node_local() -> TestResult {
    if !skip_unless_redis_resp_multinode_e2e("cross_node_tag_index_is_node_local") {
        return Ok(());
    }

    let mut cluster = DaemonCluster::start_bootstrap_with_redis(2, "redis-resp-tag-node-local")?;
    cluster.wait_for_shape(2, 2)?;
    let first_addr = cluster.redis_addr(0).expect("Redis address for node A");
    let second_addr = cluster.redis_addr(1).expect("Redis address for node B");
    let key = "node-local:tagged";

    assert_eq!(resp_set(first_addr, key, "value")?, b"+OK\r\n+OK\r\n");
    assert_eq!(
        resp_pipeline(first_addr, &[vec!["HC.TAG", key, "model"], vec!["QUIT"]],)?,
        b":1\r\n+OK\r\n"
    );
    assert_eq!(
        resp_pipeline(
            second_addr,
            &[vec!["HC.INVALIDATE_TAG", "model"], vec!["QUIT"]]
        )?,
        b":0\r\n+OK\r\n"
    );
    assert_eq!(resp_get(first_addr, key)?, b"$5\r\nvalue\r\n+OK\r\n");
    Ok(())
}

fn resp_roundtrip(addr: SocketAddr, key: &str, value: &str) -> TestResult<Vec<u8>> {
    let request = format!(
        "*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n${}\r\n{}\r\n\
         *2\r\n$3\r\nGET\r\n${}\r\n{}\r\n\
         *1\r\n$4\r\nQUIT\r\n",
        key.len(),
        key,
        value.len(),
        value,
        key.len(),
        key
    );
    resp_exchange(addr, request.as_bytes())
}

fn resp_set(addr: SocketAddr, key: &str, value: &str) -> TestResult<Vec<u8>> {
    let request = format!(
        "*3\r\n$3\r\nSET\r\n${}\r\n{}\r\n${}\r\n{}\r\n\
         *1\r\n$4\r\nQUIT\r\n",
        key.len(),
        key,
        value.len(),
        value
    );
    resp_exchange(addr, request.as_bytes())
}

fn resp_get(addr: SocketAddr, key: &str) -> TestResult<Vec<u8>> {
    let request = format!(
        "*2\r\n$3\r\nGET\r\n${}\r\n{}\r\n\
         *1\r\n$4\r\nQUIT\r\n",
        key.len(),
        key
    );
    resp_exchange(addr, request.as_bytes())
}

fn resp_set_nx_px(addr: SocketAddr, key: &str, value: &str, ttl_ms: u64) -> TestResult<Vec<u8>> {
    let ttl_ms = ttl_ms.to_string();
    let request = format!(
        "*6\r\n$3\r\nSET\r\n${}\r\n{}\r\n${}\r\n{}\r\n$2\r\nNX\r\n$2\r\nPX\r\n${}\r\n{}\r\n\
         *1\r\n$4\r\nQUIT\r\n",
        key.len(),
        key,
        value.len(),
        value,
        ttl_ms.len(),
        ttl_ms
    );
    resp_exchange(addr, request.as_bytes())
}

fn resp_pttl(addr: SocketAddr, key: &str) -> TestResult<i64> {
    let response = resp_pipeline(addr, &[vec!["PTTL", key], vec!["QUIT"]])?;
    let text = std::str::from_utf8(&response)?;
    let value = text
        .strip_prefix(':')
        .and_then(|value| value.strip_suffix("\r\n+OK\r\n"))
        .ok_or_else(|| format!("unexpected PTTL response: {text:?}"))?;
    Ok(value.parse()?)
}

fn resp_pipeline(addr: SocketAddr, commands: &[Vec<&str>]) -> TestResult<Vec<u8>> {
    let mut request = Vec::new();
    for command in commands {
        request.extend_from_slice(format!("*{}\r\n", command.len()).as_bytes());
        for argument in command {
            request.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
            request.extend_from_slice(argument.as_bytes());
            request.extend_from_slice(b"\r\n");
        }
    }
    resp_exchange(addr, &request)
}

fn resp_exchange(addr: SocketAddr, request: &[u8]) -> TestResult<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(request)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}
