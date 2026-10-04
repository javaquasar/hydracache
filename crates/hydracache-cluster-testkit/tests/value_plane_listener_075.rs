use hydracache_cluster_testkit::value_plane_listener_075::{
    ClusterListenerModel, ListenerAcceptance, ListenerBounds, ListenerError, ListenerEvent,
    ListenerEventKind,
};

fn event(partition: u32, generation: u64, watermark: u64) -> ListenerEvent {
    ListenerEvent {
        partition,
        generation,
        watermark,
        kind: ListenerEventKind::Mutation {
            key: vec![partition as u8],
            removed: false,
        },
    }
}

fn listener(backlog: usize) -> ClusterListenerModel {
    let mut listener = ClusterListenerModel::new(ListenerBounds {
        max_partitions: 2,
        max_backlog: backlog,
    })
    .unwrap();
    listener.register(0, 1, 0).unwrap();
    listener
}

#[test]
fn duplicate_watermarks_are_suppressed_and_ordered_events_are_delivered() {
    let mut listener = listener(4);
    assert_eq!(
        listener.publish(event(0, 1, 1)).unwrap(),
        ListenerAcceptance::Delivered
    );
    assert_eq!(
        listener.publish(event(0, 1, 1)).unwrap(),
        ListenerAcceptance::DuplicateSuppressed
    );
    assert_eq!(listener.drain().len(), 1);
}

#[test]
fn watermark_jump_requires_explicit_snapshot_repair() {
    let mut listener = listener(4);
    listener.publish(event(0, 1, 1)).unwrap();
    assert_eq!(
        listener.publish(event(0, 1, 3)),
        Err(ListenerError::GapRequired)
    );
    assert!(listener.requires_repair(0).unwrap());
    assert_eq!(
        listener.publish(event(0, 1, 2)),
        Err(ListenerError::GapRequired)
    );
    listener.repair(0, 1, 3).unwrap();
    listener.publish(event(0, 1, 4)).unwrap();
}

#[test]
fn migration_emits_gap_before_new_generation_events() {
    let mut listener = listener(4);
    listener.publish(event(0, 1, 1)).unwrap();
    listener.migrate(0, 2).unwrap();
    let drained = listener.drain();
    assert_eq!(drained.len(), 1);
    assert!(matches!(drained[0].kind, ListenerEventKind::Gap { .. }));
    assert_eq!(
        listener.publish(event(0, 2, 2)),
        Err(ListenerError::GapRequired)
    );
    listener.repair(0, 2, 1).unwrap();
    listener.publish(event(0, 2, 2)).unwrap();
    assert_eq!(
        listener.publish(event(0, 1, 3)),
        Err(ListenerError::StaleGeneration)
    );
}

#[test]
fn slow_consumer_overflow_collapses_to_one_gap_within_bound() {
    let mut listener = listener(2);
    listener.publish(event(0, 1, 1)).unwrap();
    listener.publish(event(0, 1, 2)).unwrap();
    assert_eq!(
        listener.publish(event(0, 1, 3)),
        Err(ListenerError::GapRequired)
    );
    assert_eq!(listener.backlog_len(), 1);
    assert!(matches!(
        listener.drain()[0].kind,
        ListenerEventKind::Gap { after_watermark: 3 }
    ));
}
