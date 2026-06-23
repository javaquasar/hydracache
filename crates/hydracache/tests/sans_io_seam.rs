use std::sync::Arc;

use hydracache::{
    ClientAck, ClientOp, ClusterNode, ClusterNodeConfig, ClusterNodeId, ClusterNodeMessage,
    ClusterStorage, HydraCache, InMemoryCluster, InMemoryClusterStorage, LogicalDuration,
    LogicalTime,
};

fn test_node() -> ClusterNode {
    ClusterNode::new(
        ClusterNodeConfig::new(
            "member-a",
            vec![
                ClusterNodeId::from("member-c"),
                ClusterNodeId::from("member-b"),
                ClusterNodeId::from("member-b"),
            ],
        )
        .heartbeat_interval(LogicalDuration::from_millis(10)),
    )
}

#[test]
fn node_is_deterministic_under_fixed_inputs() {
    let mut left = test_node();
    let mut right = test_node();

    let inputs = [
        LogicalTime::from_millis(0),
        LogicalTime::from_millis(5),
        LogicalTime::from_millis(10),
    ];

    for now in inputs {
        left.tick(now);
        right.tick(now);
    }

    let left_put = left.handle_client(ClientOp::Put {
        key: "user:1".to_owned(),
        value: b"alice".to_vec(),
    });
    let right_put = right.handle_client(ClientOp::Put {
        key: "user:1".to_owned(),
        value: b"alice".to_vec(),
    });
    assert_eq!(left_put, right_put);

    left.handle_message(
        ClusterNodeId::from("member-b"),
        ClusterNodeMessage::ReplicateInvalidate {
            key: "user:2".to_owned(),
            sequence: 99,
        },
    );
    right.handle_message(
        ClusterNodeId::from("member-b"),
        ClusterNodeMessage::ReplicateInvalidate {
            key: "user:2".to_owned(),
            sequence: 99,
        },
    );

    assert_eq!(left.peers(), right.peers());
    assert_eq!(left.take_outbound(), right.take_outbound());
    assert_eq!(left.storage_requests(), right.storage_requests());
}

#[test]
fn storage_requests_round_trip_through_explicit_driver() {
    let mut node = test_node();
    let mut storage = InMemoryClusterStorage::default();

    assert_eq!(
        node.handle_client(ClientOp::Put {
            key: "k".to_owned(),
            value: b"v1".to_vec(),
        }),
        ClientAck::Accepted { sequence: 1 }
    );
    for request in node.storage_requests() {
        let result = storage.apply(request);
        node.apply_storage_result(result);
    }
    assert_eq!(storage.get("k"), Some(&b"v1"[..]));

    let ack = node.handle_client(ClientOp::Get {
        key: "k".to_owned(),
    });
    let request_id = match ack {
        ClientAck::PendingStorage { request_id } => request_id,
        other => panic!("unexpected ack: {other:?}"),
    };
    for request in node.storage_requests() {
        let result = storage.apply(request);
        node.apply_storage_result(result);
    }
    assert_eq!(
        node.storage_result(request_id)
            .and_then(|r| r.value.as_deref()),
        Some(&b"v1"[..])
    );
}

#[tokio::test]
async fn production_path_behaviour_unchanged() {
    let cluster = Arc::new(InMemoryCluster::new("orders"));
    let member = HydraCache::member()
        .shared_cluster(cluster.clone())
        .node_id("member-a")
        .start()
        .await
        .expect("member starts");
    let client = HydraCache::client()
        .shared_cluster(cluster)
        .node_id("client-a")
        .connect()
        .await
        .expect("client starts");

    assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
    assert_eq!(client.cluster_diagnostics().unwrap().client_count, 1);
}
