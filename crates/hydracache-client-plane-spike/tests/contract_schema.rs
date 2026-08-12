use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use hydracache_client_plane_spike::contract::{ContractRelayError, PreservedMessage};
use prost::Message;
use prost_types::{DescriptorProto, FileDescriptorSet};

mod v2alpha {
    tonic::include_proto!("hydracache.client.v2alpha");
}

const DESCRIPTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/hc2_contract_descriptor.bin"));

fn message<'a>(set: &'a FileDescriptorSet, name: &str) -> &'a DescriptorProto {
    set.file
        .iter()
        .filter(|file| file.package.as_deref() == Some("hydracache.client.v2alpha"))
        .flat_map(|file| &file.message_type)
        .find(|message| message.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing authoritative message {name}"))
}

fn fields(descriptor: &DescriptorProto) -> BTreeMap<&str, i32> {
    descriptor
        .field
        .iter()
        .map(|field| {
            (
                field.name.as_deref().expect("field name"),
                field.number.expect("field number"),
            )
        })
        .collect()
}

#[test]
fn descriptor_freezes_operation_and_envelope_ids() {
    let set = FileDescriptorSet::decode(DESCRIPTOR).expect("generated descriptor");
    assert_eq!(
        fields(message(&set, "InvocationRequest")),
        BTreeMap::from([
            ("meta", 1),
            ("get", 101),
            ("put", 102),
            ("delete", 103),
            ("compare_and_set", 104),
            ("batch", 105),
            ("try_lock", 106),
            ("unlock", 107),
            ("renew_lock", 108),
            ("lock_ownership", 109),
            ("remove_if_value", 110),
        ])
    );
    assert_eq!(
        fields(message(&set, "ClientEnvelope")),
        BTreeMap::from([
            ("generation", 1),
            ("connection_generation", 2),
            ("correlation_id", 3),
            ("handshake", 16),
            ("invocation", 17),
            ("cancel", 18),
            ("subscribe", 19),
            ("session_open", 20),
            ("session_heartbeat", 21),
            ("unsubscribe", 22),
            ("session_close", 23),
        ])
    );
}

#[test]
fn descriptor_has_unique_fields_and_never_uses_reserved_ranges() {
    let set = FileDescriptorSet::decode(DESCRIPTOR).expect("generated descriptor");
    for file in &set.file {
        for descriptor in &file.message_type {
            let mut numbers = BTreeSet::new();
            let mut names = BTreeSet::new();
            for field in &descriptor.field {
                let number = field.number.expect("number");
                let name = field.name.as_deref().expect("name");
                assert!(
                    numbers.insert(number),
                    "duplicate number in {:?}",
                    descriptor.name
                );
                assert!(
                    names.insert(name),
                    "duplicate name in {:?}",
                    descriptor.name
                );
                assert!(
                    descriptor.reserved_range.iter().all(|range| {
                        number < range.start.unwrap_or_default()
                            || number >= range.end.unwrap_or_default()
                    }),
                    "field {name} reuses a reserved number in {:?}",
                    descriptor.name
                );
                assert!(
                    !descriptor
                        .reserved_name
                        .iter()
                        .any(|reserved| reserved == name),
                    "field {name} reuses a reserved name in {:?}",
                    descriptor.name
                );
            }
        }
    }
}

#[test]
fn rust_golden_handshake_is_stable() {
    let envelope = v2alpha::ClientEnvelope {
        generation: 5,
        connection_generation: 7,
        correlation_id: 0,
        message: Some(v2alpha::client_envelope::Message::Handshake(
            v2alpha::Handshake {
                generation: 5,
                client_id: "rust".into(),
                requested: vec![v2alpha::Capability::Data as i32],
                connection_generation: 7,
            },
        )),
    };
    let golden = [
        0x08, 0x05, 0x10, 0x07, 0x82, 0x01, 0x0d, 0x08, 0x05, 0x12, 0x04, 0x72, 0x75, 0x73, 0x74,
        0x1a, 0x01, 0x01, 0x20, 0x07,
    ];
    assert_eq!(envelope.encode_to_vec(), golden);
    assert_eq!(
        v2alpha::ClientEnvelope::decode(golden.as_slice()).unwrap(),
        envelope
    );
}

#[test]
fn unknown_additive_field_round_trips_exactly_until_known_fields_change() {
    let mut future = v2alpha::ClientEnvelope {
        generation: 5,
        connection_generation: 7,
        correlation_id: 9,
        message: None,
    }
    .encode_to_vec();
    // Future field 63, length-delimited "new".
    future.extend_from_slice(&[0xfa, 0x03, 0x03, b'n', b'e', b'w']);

    let mut preserved = PreservedMessage::<v2alpha::ClientEnvelope>::decode(&future).unwrap();
    assert_eq!(preserved.value().correlation_id, 9);
    assert_eq!(
        preserved.encode_preserving_unknown_fields().unwrap(),
        future
    );

    preserved.value_mut().correlation_id = 10;
    assert_eq!(
        preserved.encode_preserving_unknown_fields(),
        Err(ContractRelayError::UnknownFieldPreservationRequired)
    );
    let lossy = preserved.discard_unknown_fields_and_encode();
    assert_eq!(
        v2alpha::ClientEnvelope::decode(lossy.as_slice())
            .unwrap()
            .correlation_id,
        10
    );
    assert!(!lossy.ends_with(b"new"));
}

#[test]
fn protoc_rejects_duplicate_and_reserved_field_canaries() {
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    for fixture in ["duplicate_field_number.proto", "reserved_field_reuse.proto"] {
        let source = manifest.join("tests/fixtures").join(fixture);
        let output = Command::new(&protoc)
            .arg("--descriptor_set_out=NUL")
            .arg(format!(
                "--proto_path={}",
                source.parent().unwrap().display()
            ))
            .arg(&source)
            .output()
            .expect("run vendored protoc");
        assert!(
            !output.status.success(),
            "breaking canary {fixture} was accepted"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("already been used") || stderr.contains("reserved"),
            "{stderr}"
        );
    }
}
