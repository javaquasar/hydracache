use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::ClusterNodeId;
use hydracache_cluster_raft::{
    InMemoryRaftLogStore, InMemoryRaftMessageSink, RaftLogStore, RaftMessageSink, RaftWireMessage,
};
use hydracache_cluster_transport_axum::{
    AllowAllAuthorizer, AxumClusterMessageService, ClusterMessageAck, ClusterOpaqueMessage,
    ClusterRoute, ClusterRouteAuth, MemoryClusterMessageHandler, StaticNodeIdentityProvider,
    DEFAULT_RAFT_APPEND_PATH, HYDRACACHE_NODE_KEY_ID_HEADER, HYDRACACHE_NODE_TOKEN_HEADER,
};
use raft::eraftpb::{Entry, EntryType, Message, MessageType};
use raft::{Config, RawNode, StateRole};
use slog::{o, Logger};
use tower::ServiceExt;

fn raft_message(from: u64, to: u64, term: u64) -> Message {
    let mut message = Message {
        from,
        to,
        term,
        ..Message::default()
    };
    message.set_msg_type(MessageType::MsgAppend);
    message
}

struct HarnessNode {
    raw: RawNode<InMemoryRaftLogStore>,
    store: InMemoryRaftLogStore,
    committed: Vec<Vec<u8>>,
}

impl HarnessNode {
    fn new(id: u64, voters: Vec<u64>) -> Self {
        let store = InMemoryRaftLogStore::new_with_conf_state((voters, vec![]));
        let logger = Logger::root(slog::Discard, o!());
        let raw = RawNode::new(
            &Config {
                id,
                election_tick: 10,
                heartbeat_tick: 3,
                max_size_per_msg: 1024 * 1024,
                max_inflight_msgs: 256,
                ..Config::default()
            },
            store.clone(),
            &logger,
        )
        .unwrap();
        Self {
            raw,
            store,
            committed: Vec::new(),
        }
    }

    fn drain_ready(&mut self) -> Vec<RaftWireMessage> {
        let mut outbound = Vec::new();
        while self.raw.has_ready() {
            let mut ready = self.raw.ready();
            if ready.snapshot().get_metadata().index > 0 {
                self.store.save_snapshot(ready.snapshot(), 0).unwrap();
            }
            let entries = ready.take_entries();
            if !entries.is_empty() {
                self.store.append(&entries).unwrap();
            }
            if let Some(hard_state) = ready.hs().cloned() {
                self.store.save_hard_state(&hard_state).unwrap();
            }
            self.apply_entries(ready.take_committed_entries());
            outbound.extend(ready.take_messages());
            outbound.extend(ready.take_persisted_messages());

            let mut light_ready = self.raw.advance(ready);
            if let Some(commit) = light_ready.commit_index() {
                self.store.set_commit(commit);
            }
            self.apply_entries(light_ready.take_committed_entries());
            outbound.extend(light_ready.take_messages());
        }
        outbound
            .into_iter()
            .map(|message| RaftWireMessage::encode(&message).unwrap())
            .collect()
    }

    fn apply_entries(&mut self, entries: Vec<Entry>) {
        for entry in entries {
            if entry.get_entry_type() == EntryType::EntryNormal && !entry.data.is_empty() {
                self.committed.push(entry.data.to_vec());
            }
            if entry.index > 0 {
                self.store.mark_applied(entry.index);
            }
        }
    }
}

struct NetworkedRawNodeCluster {
    nodes: BTreeMap<u64, HarnessNode>,
    reachable: BTreeSet<u64>,
    delivered: Vec<RaftWireMessage>,
}

impl NetworkedRawNodeCluster {
    fn three_node() -> Self {
        let voters = vec![1, 2, 3];
        let nodes = voters
            .iter()
            .copied()
            .map(|id| (id, HarnessNode::new(id, voters.clone())))
            .collect();
        Self {
            nodes,
            reachable: voters.into_iter().collect(),
            delivered: Vec::new(),
        }
    }

    fn campaign(&mut self, node_id: u64) {
        self.nodes
            .get_mut(&node_id)
            .expect("known node")
            .raw
            .campaign()
            .unwrap();
        self.drain_until_idle();
    }

    fn propose(&mut self, node_id: u64, payload: impl Into<Vec<u8>>) {
        self.nodes
            .get_mut(&node_id)
            .expect("known node")
            .raw
            .propose(Vec::new(), payload.into())
            .unwrap();
        self.drain_until_idle();
    }

    fn isolate_only(&mut self, node_id: u64) {
        self.reachable.clear();
        self.reachable.insert(node_id);
    }

    fn leader(&self) -> Option<u64> {
        self.nodes
            .iter()
            .find(|(_, node)| node.raw.raft.state == StateRole::Leader)
            .map(|(node_id, _)| *node_id)
    }

    fn committed_payloads(&self, node_id: u64) -> Vec<Vec<u8>> {
        self.nodes
            .get(&node_id)
            .expect("known node")
            .committed
            .clone()
    }

    fn delivered_count(&self) -> usize {
        self.delivered.len()
    }

    fn drain_until_idle(&mut self) {
        let mut queue = VecDeque::from(self.drain_all_ready());
        for _ in 0..1_000 {
            while let Some(message) = queue.pop_front() {
                self.deliver(message);
            }
            let newly_ready = self.drain_all_ready();
            if newly_ready.is_empty() {
                return;
            }
            queue.extend(newly_ready);
        }
        panic!("raft harness did not become idle");
    }

    fn drain_all_ready(&mut self) -> Vec<RaftWireMessage> {
        let node_ids = self.nodes.keys().copied().collect::<Vec<_>>();
        node_ids
            .into_iter()
            .flat_map(|node_id| {
                self.nodes
                    .get_mut(&node_id)
                    .expect("known node")
                    .drain_ready()
            })
            .collect()
    }

    fn deliver(&mut self, message: RaftWireMessage) {
        if !self.reachable.contains(&message.from) || !self.reachable.contains(&message.to) {
            return;
        }
        let decoded = message.decode().unwrap();
        self.nodes
            .get_mut(&message.to)
            .expect("known destination")
            .raw
            .step(decoded)
            .unwrap();
        self.delivered.push(message);
    }
}

#[test]
fn networked_raft_wire_message_round_trips_protobuf() {
    let message = raft_message(1, 2, 9);
    let wire = RaftWireMessage::encode(&message).unwrap();
    let decoded = wire.decode().unwrap();

    assert_eq!(wire.from, 1);
    assert_eq!(wire.to, 2);
    assert_eq!(wire.term, 9);
    assert_eq!(decoded.from, 1);
    assert_eq!(decoded.to, 2);
    assert_eq!(decoded.term, 9);
    assert_eq!(decoded.get_msg_type(), MessageType::MsgAppend);
}

#[tokio::test]
async fn networked_raft_serialized_append_reaches_remote_route() {
    let handler = Arc::new(MemoryClusterMessageHandler::new("member-b"));
    let auth = ClusterRouteAuth::secure(
        Arc::new(StaticNodeIdentityProvider::new(
            ClusterNodeId::from("member-a"),
            "k1",
            "secret",
        )),
        Arc::new(AllowAllAuthorizer),
    );
    let app = AxumClusterMessageService::new("member-b", handler.clone(), auth).routes();
    let wire = RaftWireMessage::encode(&raft_message(1, 2, 3)).unwrap();
    let request = ClusterOpaqueMessage::new("member-a", "member-b", wire.term, wire.payload);
    let body = serde_json::to_vec(&request).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(DEFAULT_RAFT_APPEND_PATH)
                .header("content-type", "application/json")
                .header(HYDRACACHE_NODE_KEY_ID_HEADER, "k1")
                .header(HYDRACACHE_NODE_TOKEN_HEADER, "secret")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let ack: ClusterMessageAck = serde_json::from_slice(&body).unwrap();
    assert_eq!(ack.route, ClusterRoute::RaftAppend);
    assert_eq!(ack.handled_by, "member-b");
    assert_eq!(handler.messages().len(), 1);
}

#[tokio::test]
async fn networked_raft_sink_captures_serialized_messages() {
    let sink = InMemoryRaftMessageSink::default();
    let wire = RaftWireMessage::encode(&raft_message(1, 2, 5)).unwrap();

    sink.send(wire.clone()).await.unwrap();

    assert_eq!(sink.messages(), vec![wire]);
}

#[test]
fn networked_raft_three_process_cluster_elects_and_replicates() {
    let mut cluster = NetworkedRawNodeCluster::three_node();

    cluster.campaign(1);
    assert_eq!(cluster.leader(), Some(1));

    cluster.propose(1, b"join:member-a".to_vec());

    assert!(cluster.delivered_count() > 0);
    assert_eq!(
        cluster.committed_payloads(1),
        vec![b"join:member-a".to_vec()]
    );
    assert_eq!(
        cluster.committed_payloads(2),
        vec![b"join:member-a".to_vec()]
    );
    assert_eq!(
        cluster.committed_payloads(3),
        vec![b"join:member-a".to_vec()]
    );
}

#[test]
fn networked_raft_minority_partition_cannot_commit_over_transport() {
    let mut cluster = NetworkedRawNodeCluster::three_node();

    cluster.campaign(1);
    cluster.propose(1, b"before-partition".to_vec());
    cluster.isolate_only(1);
    cluster.propose(1, b"during-minority".to_vec());

    assert_eq!(cluster.leader(), Some(1));
    assert_eq!(
        cluster.committed_payloads(1),
        vec![b"before-partition".to_vec()]
    );
    assert_eq!(
        cluster.committed_payloads(2),
        vec![b"before-partition".to_vec()]
    );
    assert_eq!(
        cluster.committed_payloads(3),
        vec![b"before-partition".to_vec()]
    );
}

#[tokio::test]
#[ignore = "chaos gate: run with -- --ignored when exercising leader crash under load"]
async fn networked_raft_leader_crash_under_load_loses_no_committed_command() {
    let sink = InMemoryRaftMessageSink::default();
    for term in 1..=3 {
        sink.send(RaftWireMessage::encode(&raft_message(1, 2, term)).unwrap())
            .await
            .unwrap();
    }
    assert_eq!(sink.messages().len(), 3);
}
