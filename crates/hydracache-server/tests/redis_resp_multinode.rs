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

fn resp_roundtrip(addr: SocketAddr, key: &str, value: &str) -> TestResult<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
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
    stream.write_all(request.as_bytes())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}
