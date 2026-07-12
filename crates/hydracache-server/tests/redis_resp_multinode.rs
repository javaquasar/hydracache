mod support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use support::daemon_cluster::{skip_unless_daemon_process_e2e, DaemonCluster, TestResult};

#[test]
fn multinode_resp_facade_roundtrip_survives_node_restart_or_drain() -> TestResult {
    if !skip_unless_daemon_process_e2e(
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
    if !skip_unless_daemon_process_e2e("multinode_resp_facade_documents_node_local_state") {
        return Ok(());
    }

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
    if !skip_unless_daemon_process_e2e("multinode_resp_lock_subset_is_single_endpoint_only") {
        return Ok(());
    }

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

fn resp_exchange(addr: SocketAddr, request: &[u8]) -> TestResult<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(request)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}
