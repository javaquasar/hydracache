use hydracache_cluster_testkit::value_plane_loopback_075::{
    LoopbackError, LoopbackFault, LoopbackFrame, LoopbackTransport,
};

#[test]
fn loopback_delays_duplicates_and_drops_deterministically() {
    let mut transport = LoopbackTransport::new(8, 2).unwrap();
    transport
        .send(LoopbackFrame::new(2, 1, vec![1]), LoopbackFault::Drop)
        .unwrap();
    transport
        .send(LoopbackFrame::new(2, 2, vec![2]), LoopbackFault::Duplicate)
        .unwrap();
    transport
        .send(LoopbackFrame::new(2, 3, vec![3]), LoopbackFault::Delay(2))
        .unwrap();
    assert_eq!(transport.receive().unwrap().unwrap().sequence, 2);
    assert_eq!(transport.receive().unwrap().unwrap().sequence, 2);
    assert!(transport.receive().unwrap().is_none());
    transport.advance();
    transport.advance();
    assert_eq!(transport.receive().unwrap().unwrap().sequence, 3);
    assert!(transport.trace().contains(&"dropped"));
}

#[test]
fn stale_oversized_partial_and_corrupt_frames_fail_loud() {
    assert!(LoopbackTransport::new(0, 1).is_err());
    let mut transport = LoopbackTransport::new(2, 2).unwrap();
    assert_eq!(
        transport.send(LoopbackFrame::new(1, 1, vec![1]), LoopbackFault::Deliver),
        Err(LoopbackError::StaleGeneration)
    );
    assert_eq!(
        transport.send(
            LoopbackFrame::new(2, 1, vec![1, 2, 3]),
            LoopbackFault::Deliver
        ),
        Err(LoopbackError::PayloadTooLarge)
    );
    transport
        .send(LoopbackFrame::new(2, 2, vec![1, 2]), LoopbackFault::Partial)
        .unwrap();
    assert_eq!(transport.receive(), Err(LoopbackError::CorruptFrame));
    transport
        .send(LoopbackFrame::new(2, 3, vec![1]), LoopbackFault::Corrupt)
        .unwrap();
    assert_eq!(transport.receive(), Err(LoopbackError::CorruptFrame));
}
