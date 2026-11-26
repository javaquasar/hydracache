use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use hydracache::{
    ClusterCandidate, ClusterControlPlane, ClusterEpoch, InMemoryReplicatedValueStore, PartitionId,
    ReplicatedValueRecord, ReplicatedValueStore, ReplicationConfig,
};
use hydracache_client_protocol::Namespace;
use hydracache_client_transport_axum::ClientSurfaceDispatcher;
use hydracache_cluster_raft::{InMemoryRaftMessageSink, RaftMetadataRuntime};
use hydracache_redis_compat::{
    redis_structured_key_to_bytes, RedisListenerConfig, RedisRespServer,
};
use hydracache_server::imap_owner_read::RaftOwnerReadBackend;
use hydracache_server::imap_replicated_read::{
    ReplicaReadError, ReplicaReadPrefix, ReplicaReadScope, ReplicatedReadValues, D2_PARTITION_COUNT,
};
use hydracache_server::imap_topology::PartitionTopology;
use tokio::net::TcpListener;

const KEY: &[u8] = b"\0order\xff";
const VALUE: &[u8] = b"persisted\0\xff";

fn scope() -> ReplicaReadScope {
    ReplicaReadScope::new(
        "redis".into(),
        Namespace::new("redis").unwrap(),
        NonZeroU64::new(1).unwrap(),
        redis_structured_key_to_bytes,
    )
}

fn prefix(epoch: ClusterEpoch, applied: u64) -> ReplicaReadPrefix {
    ReplicaReadPrefix {
        epoch,
        applied,
        required: 8,
    }
}

fn memory_with(record: ReplicatedValueRecord, key: String) -> InMemoryReplicatedValueStore {
    let mut store = InMemoryReplicatedValueStore::default();
    store.upsert(key, record).unwrap();
    store
}

fn start_resp(
    listener: TcpListener,
    dispatcher: Arc<ClientSurfaceDispatcher>,
) -> tokio::task::JoinHandle<()> {
    let resp = Arc::new(
        RedisRespServer::from_dispatcher(dispatcher, RedisListenerConfig::default()).unwrap(),
    );
    assert!(resp.local_state().is_none());
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let resp = resp.clone();
            tokio::spawn(async move {
                let _ = resp.serve_connection(stream).await;
            });
        }
    })
}

fn resp(address: SocketAddr, arguments: &[&[u8]]) -> io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(500))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    write!(stream, "*{}\r\n", arguments.len())?;
    for arg in arguments {
        write!(stream, "${}\r\n", arg.len())?;
        stream.write_all(arg)?;
        stream.write_all(b"\r\n")?;
    }
    let mut reader = BufReader::new(stream);
    let mut response = Vec::new();
    reader.read_until(b'\n', &mut response)?;
    if response.first() == Some(&b'$') {
        let length: i64 = std::str::from_utf8(&response[1..])
            .map_err(io::Error::other)?
            .trim()
            .parse()
            .map_err(io::Error::other)?;
        if length >= 0 {
            let start = response.len();
            response.resize(start + length as usize + 2, 0);
            reader.read_exact(&mut response[start..])?;
        }
    }
    Ok(response)
}

fn expected_value() -> Vec<u8> {
    [format!("${}\r\n", VALUE.len()).as_bytes(), VALUE, b"\r\n"].concat()
}

#[test]
fn rejects_misplaced_corrupt_or_regressed_replica_images() {
    let epoch = ClusterEpoch::new(3);
    let (partition, key) = scope().locate(KEY.to_vec()).unwrap();
    let values = ReplicatedReadValues::new(scope());
    let record = ReplicatedValueRecord::value(partition, 8, epoch, VALUE);
    let good = memory_with(record.clone(), key.clone());
    values.install(partition, prefix(epoch, 8), good).unwrap();

    for (key, record) in [
        ("malformed".to_owned(), record.clone()),
        (
            key.clone(),
            ReplicatedValueRecord::value(PartitionId::new(271), 8, epoch, VALUE),
        ),
        (
            key.clone(),
            ReplicatedValueRecord::value(partition, 9, epoch, VALUE),
        ),
        (
            key.clone(),
            ReplicatedValueRecord::value(partition, 8, ClusterEpoch::new(4), VALUE),
        ),
        (
            key.clone(),
            ReplicatedValueRecord {
                version: 7,
                ..record.clone()
            },
        ),
    ] {
        assert!(matches!(
            values.install(partition, prefix(epoch, 8), memory_with(record, key)),
            Err(ReplicaReadError::InvalidSnapshot)
        ));
    }
    let empty = InMemoryReplicatedValueStore::default();
    assert!(matches!(
        values.install(partition, prefix(epoch, 8), empty.clone()),
        Err(ReplicaReadError::Regressed)
    ));
    assert!(matches!(
        values.install(partition, prefix(epoch, 7), empty.clone()),
        Err(ReplicaReadError::Regressed)
    ));
    assert!(matches!(
        values.install(partition, prefix(ClusterEpoch::new(2), 8), empty.clone()),
        Err(ReplicaReadError::Regressed)
    ));
    assert!(matches!(
        values.install(
            partition,
            ReplicaReadPrefix {
                required: 7,
                ..prefix(ClusterEpoch::new(4), 8)
            },
            empty
        ),
        Err(ReplicaReadError::Regressed)
    ));
    assert_eq!(values.store_lookups(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resp_get_reads_the_replicated_store_only_after_a_published_prefix() {
    let raft = Arc::new(RaftMetadataRuntime::single_node("replica-075", 1).unwrap());
    raft.join_member(ClusterCandidate::member("local"))
        .await
        .unwrap();
    let snapshot = raft.partition_snapshot().unwrap();
    let epoch = snapshot.epoch;
    let replication = ReplicationConfig::local_first();
    let (partition, key) = scope().locate(KEY.to_vec()).unwrap();
    let local = PartitionTopology::from_snapshot(snapshot, D2_PARTITION_COUNT, replication)
        .unwrap()
        .placement(partition)
        .unwrap()
        .primary;
    let values = ReplicatedReadValues::new(scope());
    let backend = RaftOwnerReadBackend::new(
        raft,
        Arc::new(InMemoryRaftMessageSink::default()),
        values.clone(),
        local,
        D2_PARTITION_COUNT,
        replication,
    );
    let dispatcher = Arc::new(
        ClientSurfaceDispatcher::from_backend(Default::default(), Arc::new(backend)).unwrap(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = start_resp(listener, dispatcher);
    let unavailable = b"-ERR backend unavailable\r\n";
    assert_eq!(resp(address, &[b"GET", KEY]).unwrap(), unavailable);
    values
        .install(
            partition,
            prefix(epoch, 7),
            InMemoryReplicatedValueStore::default(),
        )
        .unwrap();
    assert_eq!(resp(address, &[b"GET", KEY]).unwrap(), unavailable);
    assert_eq!(values.store_lookups(), 0);
    let mut store = memory_with(
        ReplicatedValueRecord::value(partition, 8, epoch, VALUE),
        key.clone(),
    );
    let missing = (0u32..)
        .map(|i| i.to_le_bytes())
        .find(|key| scope().locate(key.to_vec()).unwrap().0 == partition)
        .unwrap();
    let (empty_partition, empty_key) = scope().locate(Vec::new()).unwrap();
    let empty_record = ReplicatedValueRecord::value(empty_partition, 8, epoch, Vec::new());
    if empty_partition == partition {
        store.upsert(empty_key, empty_record).unwrap();
    } else {
        values
            .install(
                empty_partition,
                prefix(epoch, 8),
                memory_with(empty_record, empty_key),
            )
            .unwrap();
    }
    values.install(partition, prefix(epoch, 8), store).unwrap();
    assert_eq!(resp(address, &[b"GET", KEY]).unwrap(), expected_value());
    assert_eq!(resp(address, &[b"GET", &missing]).unwrap(), b"$-1\r\n");
    assert_eq!(resp(address, &[b"GET", b""]).unwrap(), b"$0\r\n\r\n");
    assert_eq!(
        resp(address, &[b"SET", KEY, b"would-be-local"]).unwrap(),
        unavailable
    );
    assert_eq!(values.store_lookups(), 3);
    values
        .install(
            partition,
            prefix(epoch, 9),
            memory_with(
                ReplicatedValueRecord::tombstone(partition, 9, epoch, None),
                key,
            ),
        )
        .unwrap();
    assert_eq!(resp(address, &[b"GET", KEY]).unwrap(), b"$-1\r\n");
    assert_eq!(values.store_lookups(), 4);
    server.abort();
}

#[cfg(target_os = "linux")]
#[path = "support/imap_replicated_read_process.rs"]
mod processes;
