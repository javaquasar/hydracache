use chitchat::transport::{ChannelTransport, Transport};
use chitchat::{
    Chitchat, ChitchatConfig, ChitchatId, ChitchatMessage, Deserializable, Serializable,
};
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::watch;

fn config(port: u16) -> ChitchatConfig {
    let addr = ([127, 0, 0, 1], port).into();
    ChitchatConfig {
        chitchat_id: ChitchatId::new(format!("consumer-{port}"), 1, addr),
        cluster_id: "hc074-owned-source".into(),
        gossip_interval: Duration::from_millis(50),
        listen_addr: addr,
        seed_nodes: Vec::new(),
        failure_detector_config: Default::default(),
        marked_for_deletion_grace_period: Duration::from_secs(100),
        catchup_callback: None,
        extra_liveness_predicate: None,
    }
}

#[test]
fn original_library_alias_and_wire_are_stable() {
    assert!(std::any::type_name::<Chitchat>().starts_with("chitchat::"));
    let msg = ChitchatMessage::BadCluster;
    let wire = msg.serialize_to_vec();
    assert_eq!(wire, [0x53, 0xb0, 0, 3]);
    let mut view = wire.as_slice();
    assert_eq!(ChitchatMessage::deserialize(&mut view).unwrap(), msg);
    assert!(view.is_empty());
}

#[test]
fn malformed_and_future_protocol_refuse() {
    for bytes in [
        &[][..],
        &[0x53][..],
        &[0x53, 0xb0, 255, 3][..],
        &[0x53, 0xb0, 0, 254][..],
    ] {
        assert!(ChitchatMessage::deserialize(&mut &bytes[..]).is_err());
    }
}

#[test]
fn binary_unicode_scalar_projection_round_trips() {
    let value = "\0Hydra-ключ-🐙";
    let mut wire = value.serialize_to_vec();
    wire.push(19);
    let mut view = wire.as_slice();
    assert_eq!(String::deserialize(&mut view).unwrap(), value);
    assert_eq!(view, [19]);
}

#[tokio::test]
async fn public_state_versions_and_deletion_remain_original() {
    let (_tx, rx) = watch::channel(HashSet::new());
    let mut state = Chitchat::with_chitchat_id_and_seeds(config(18001), rx, Vec::new());
    assert_eq!(state.cluster_id(), "hc074-owned-source");
    let node = state.self_node_state();
    node.set("key", "one");
    let version = node.max_version();
    node.set("key", "two");
    assert!(node.max_version() > version);
    assert_eq!(node.get("key"), Some("two"));
    node.delete("key");
    assert_eq!(node.get("key"), None);
}

#[tokio::test]
async fn actual_channel_transport_delivers_original_wire_surface() {
    let transport = ChannelTransport::with_mtu(65507);
    let a = ([127, 0, 0, 1], 18101).into();
    let b = ([127, 0, 0, 1], 18102).into();
    let mut sender = transport.open(a).await.unwrap();
    let mut receiver = transport.open(b).await.unwrap();
    sender.send(b, ChitchatMessage::BadCluster).await.unwrap();
    let (peer, message) = receiver.recv().await.unwrap();
    assert_eq!(peer, a);
    assert_eq!(message, ChitchatMessage::BadCluster);
    assert_eq!(transport.statistics().num_messages_total, 1);
}

#[tokio::test]
async fn original_spawn_and_owned_shutdown_remain_usable() {
    let transport = ChannelTransport::with_mtu(65507);
    let handle = chitchat::spawn_chitchat(
        config(18201),
        vec![("hello".into(), "world".into())],
        &transport,
    )
    .await
    .unwrap();
    assert_eq!(
        handle
            .with_chitchat(|c| c.self_node_state().get("hello").map(str::to_owned))
            .await
            .as_deref(),
        Some("world")
    );
    handle.shutdown().await.unwrap();
}
