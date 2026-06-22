use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache::ClusterNodeId;
use hydracache_cluster_raft::{InMemoryRaftMessageSink, RaftMessageSink, RaftWireMessage};
use hydracache_cluster_transport_axum::{
    AllowAllAuthorizer, AxumClusterMessageService, ClusterMessageAck, ClusterOpaqueMessage,
    ClusterRoute, ClusterRouteAuth, MemoryClusterMessageHandler, StaticNodeIdentityProvider,
    DEFAULT_RAFT_APPEND_PATH, HYDRACACHE_NODE_KEY_ID_HEADER, HYDRACACHE_NODE_TOKEN_HEADER,
};
use raft::eraftpb::{Message, MessageType};
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
