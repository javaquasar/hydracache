use super::*;
fn binding() -> RootBindingClaims {
    RootBindingClaims {
        attempt_sha256: [7; 32],
        root_device: 11,
        root_inode: 13,
        deadline_boottime_ns: 17,
        original_context_sha256: [19; 32],
    }
}
fn ack() -> [u8; PACKET_BYTES] {
    PacketClaims {
        kind: Kind::Armed,
        binding: binding(),
    }
    .encode()
    .unwrap()
}
#[test]
fn root_enrollment_all_kinds_roundtrip_with_exact_offsets() {
    for kind in [Kind::Enroll, Kind::Armed, Kind::Refused, Kind::Revoking] {
        let packet = PacketClaims {
            kind,
            binding: binding(),
        };
        let raw = packet.encode().unwrap();
        assert_eq!(&raw[..8], b"HCRG0742");
        assert_eq!(raw[8], kind as u8);
        assert_eq!(&raw[9..16], &[0; 7]);
        assert_eq!(&raw[48..56], &11u64.to_le_bytes());
        assert_eq!(&raw[56..64], &13u64.to_le_bytes());
        assert_eq!(&raw[64..72], &17u64.to_le_bytes());
        assert_eq!(inspect_packet(&raw), Ok(packet));
    }
}
#[test]
fn root_enrollment_invalid_shapes_versions_fields_and_old_packets_refuse() {
    let raw = ack();
    for len in 0..PACKET_BYTES {
        assert!(inspect_packet(&raw[..len]).is_err());
    }
    let mut extra = raw.to_vec();
    extra.push(0);
    assert!(inspect_packet(&extra).is_err());
    for index in (0..8).chain(9..16) {
        let mut changed = raw;
        changed[index] ^= 1;
        assert!(inspect_packet(&changed).is_err());
    }
    for kind in 0..=255 {
        let mut changed = raw;
        changed[8] = kind;
        assert_eq!(inspect_packet(&changed).is_ok(), (1..=4).contains(&kind));
    }
    assert!(inspect_packet(b"HCPG0741\x01\0\0\0\0\0\0\0").is_err());
    for range in [16..48, 48..56, 56..64, 64..72, 72..104] {
        let mut changed = raw;
        changed[range].fill(0);
        assert!(inspect_packet(&changed).is_err());
    }
    let mut changed = raw;
    changed[64..72].copy_from_slice(&(i64::MAX as u64 + 1).to_le_bytes());
    assert!(inspect_packet(&changed).is_err());
}
#[test]
fn root_enrollment_send_uncertainty_and_ack_are_sticky_without_authority() {
    for outcome in ["success", "failed-send", "lost-ack", "bad-ack"] {
        let mut exchange = ClaimExchange::new(binding()).unwrap();
        assert_eq!(
            inspect_packet(&exchange.prepare_first_send().unwrap())
                .unwrap()
                .kind,
            Kind::Enroll
        );
        assert!(exchange.consumed && !exchange.acknowledged);
        match outcome {
            "success" => exchange.inspect_armed_ack_claim(&ack()).unwrap(),
            "bad-ack" => assert!(exchange.inspect_armed_ack_claim(b"bad").is_err()),
            _ => (),
        }
        assert!(exchange.prepare_first_send().is_err());
        assert!(exchange.inspect_armed_ack_claim(&ack()).is_err());
        assert!(exchange.refused);
    }
    let mut exchange = ClaimExchange::new(binding()).unwrap();
    assert!(exchange.inspect_armed_ack_claim(&ack()).is_err());
    assert!(exchange.prepare_first_send().is_err());
    let mut exchange = ClaimExchange::new(binding()).unwrap();
    exchange.prepare_first_send().unwrap();
    exchange.inspect_armed_ack_claim(&ack()).unwrap();
    assert!(exchange.inspect_armed_ack_claim(&ack()).is_err());
}
#[test]
fn root_enrollment_every_ack_byte_mutation_refuses_and_cannot_restore() {
    for index in 0..PACKET_BYTES {
        let mut exchange = ClaimExchange::new(binding()).unwrap();
        exchange.prepare_first_send().unwrap();
        let mut changed = ack();
        changed[index] ^= 1;
        assert!(
            exchange.inspect_armed_ack_claim(&changed).is_err(),
            "byte {index}"
        );
        assert!(exchange.inspect_armed_ack_claim(&ack()).is_err());
        assert!(exchange.prepare_first_send().is_err());
    }
}
#[test]
fn root_enrollment_seeded_bindings_roundtrip_without_deadline_renewal() {
    let seed = 0x074c_2019_u64;
    eprintln!("root enrollment seed={seed:#x}");
    let mut random = seed;
    for _ in 0..256 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let mut original = binding();
        original.root_device = random | 1;
        original.root_inode = random.rotate_left(7) | 1;
        original.deadline_boottime_ns = (random & i64::MAX as u64) | 1;
        original.attempt_sha256[..8].copy_from_slice(&random.to_le_bytes());
        let packet = PacketClaims {
            kind: Kind::Enroll,
            binding: original,
        };
        let inspected = inspect_packet(&packet.encode().unwrap()).unwrap();
        assert_eq!(inspected, packet);
        assert_eq!(
            inspected.binding.deadline_boottime_ns,
            original.deadline_boottime_ns
        );
    }
    let mut zero = binding();
    zero.root_inode = 0;
    assert!(ClaimExchange::new(zero).is_err());
}
