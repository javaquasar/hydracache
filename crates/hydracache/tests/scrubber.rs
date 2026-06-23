use std::collections::BTreeMap;

use hydracache::{
    ChecksummedReplicatedValueRecord, ClusterEpoch, PartitionId, ReplicatedValueRecord,
    ScrubErrorKind, Scrubber,
};

#[test]
fn scrubber_corrupt_block_is_detected_and_repaired_from_peer() {
    let key = "user:42".to_owned();
    let good_record = ReplicatedValueRecord::value(
        PartitionId::new(1),
        7,
        ClusterEpoch::new(2),
        b"good".to_vec(),
    );
    let good = ChecksummedReplicatedValueRecord::seal(good_record.clone());
    let corrupt_payload = ReplicatedValueRecord::value(
        PartitionId::new(1),
        7,
        ClusterEpoch::new(2),
        b"bad".to_vec(),
    );
    let corrupt = ChecksummedReplicatedValueRecord::from_parts(
        good.checksum_format,
        corrupt_payload,
        good.checksum,
    );
    let mut primary = BTreeMap::from([(key.clone(), corrupt)]);
    let peer = BTreeMap::from([(key.clone(), good.clone())]);

    let report = Scrubber::default().scrub_replicated_values(&mut primary, &[peer]);

    assert!(report.is_ok(), "{report:?}");
    assert_eq!(report.checked, 1);
    assert_eq!(report.corrupt, 1);
    assert_eq!(report.repaired, 1);
    assert_eq!(
        Scrubber::verified_get(&primary, &key).unwrap(),
        Some(good_record)
    );
}

#[test]
fn scrubber_unrepairable_corruption_is_reported_not_served() {
    let key = "user:99".to_owned();
    let good_record = ReplicatedValueRecord::value(
        PartitionId::new(1),
        8,
        ClusterEpoch::new(2),
        b"good".to_vec(),
    );
    let good = ChecksummedReplicatedValueRecord::seal(good_record);
    let corrupt_payload = ReplicatedValueRecord::value(
        PartitionId::new(1),
        8,
        ClusterEpoch::new(2),
        b"bad".to_vec(),
    );
    let corrupt = ChecksummedReplicatedValueRecord::from_parts(
        good.checksum_format,
        corrupt_payload,
        good.checksum,
    );
    let mut primary = BTreeMap::from([(key.clone(), corrupt)]);

    let report = Scrubber::default().scrub_replicated_values(&mut primary, &[]);

    assert_eq!(report.corrupt, 1);
    assert_eq!(report.repaired, 0);
    assert_eq!(report.unrepairable, 1);
    let error = Scrubber::verified_get(&primary, &key).expect_err("corruption is not served");
    assert_eq!(error.key, key);
    assert!(matches!(
        error.kind,
        ScrubErrorKind::ChecksumMismatch { .. }
    ));
}
