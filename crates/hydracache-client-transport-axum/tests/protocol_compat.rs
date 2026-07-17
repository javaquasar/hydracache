use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_protocol::{
    ClientFrame, ClientProtocolError, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientWireMessage, ConditionalPutCondition, Namespace, StructuredKey,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const V2_REQUEST: &str = include_str!("fixtures/protocol_v2_evict_request.hex");
const V2_RESPONSE: &str = include_str!("fixtures/protocol_v2_evict_response.hex");
const V3_REQUEST: &str = include_str!("fixtures/protocol_v3_evict_request.hex");
const V3_RESPONSE: &str = include_str!("fixtures/protocol_v3_evict_response.hex");
const PROVENANCE: &str = include_str!("fixtures/protocol_compat_provenance.json");
const MAX_FRAME_BYTES: usize = 1024 * 1024;
const FIXTURE_CODEC: &str = "postcard length-prefixed ClientFrame";
const FIXTURE_GENERATOR: &str =
    "hydracache-client-protocol ClientRequest::EvictRegion and ClientResponse::Evicted";

#[derive(Clone, Copy)]
struct FrozenFixtureProvenance {
    wire_version: u16,
    source_ref: &'static str,
    source_commit: &'static str,
    request_file: &'static str,
    request_sha256: &'static str,
    response_file: &'static str,
    response_sha256: &'static str,
    request_hex: &'static str,
    response_hex: &'static str,
}

const FROZEN_FIXTURES: [FrozenFixtureProvenance; 2] = [
    FrozenFixtureProvenance {
        wire_version: 2,
        source_ref: "v0.62.0",
        source_commit: "bcf4b8b01448cc3b4a566bf4ed63444a2c90cbed",
        request_file: "protocol_v2_evict_request.hex",
        request_sha256: "21dbb8aa000a7197f9db33ad7c484c092dbd56e78df6bff176e03964cf232def",
        response_file: "protocol_v2_evict_response.hex",
        response_sha256: "08f8c8ec0cc9b5de0632f89ec67fa3d818c67324b38fb4c094df0f46574448cc",
        request_hex: V2_REQUEST,
        response_hex: V2_RESPONSE,
    },
    FrozenFixtureProvenance {
        wire_version: 3,
        source_ref: "667b2e6",
        source_commit: "667b2e635fee288ef3e40632954d215e20aa5a1b",
        request_file: "protocol_v3_evict_request.hex",
        request_sha256: "7dee87679b956fbaae9e33dc878b8b3bcef8e109d58644ca4555efe998337805",
        response_file: "protocol_v3_evict_response.hex",
        response_sha256: "595ed7c8e428ad67001281f567c5d2179ba431683df4d5ac94a409b0b15c6b17",
        request_hex: V3_REQUEST,
        response_hex: V3_RESPONSE,
    },
];

fn decode_hex(input: &str) -> Vec<u8> {
    let compact: String = input.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert_eq!(compact.len() % 2, 0, "fixture hex must contain whole bytes");
    compact
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("ASCII hex fixture");
            u8::from_str_radix(pair, 16).expect("valid hex fixture")
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn assert_published_fixture_roundtrip(
    wire_version: u16,
    request_hex: &str,
    response_hex: &str,
) {
    let request_bytes = decode_hex(request_hex);
    let request_frame = ClientFrame::decode(&request_bytes, MAX_FRAME_BYTES).expect("old frame");
    assert_eq!(request_frame.protocol_version(), wire_version);
    let ClientWireMessage::Request(request) = request_frame
        .decode_message()
        .expect("old request decodes with its historical schema")
    else {
        panic!("expected request fixture");
    };
    assert_eq!(request.protocol_version, wire_version);
    assert!(matches!(request.request, ClientRequest::EvictRegion { .. }));
    let reencoded = ClientFrame::from_message_with_version(
        wire_version,
        &ClientWireMessage::Request(request.clone()),
    )
    .unwrap()
    .encode()
    .unwrap();
    assert_eq!(reencoded.as_ref(), request_bytes);

    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "legacy-client")
                .header(HYDRACACHE_TENANT_HEADER, "legacy-tenant")
                .body(Body::from(request_bytes))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let response_bytes = to_bytes(response.into_body(), MAX_FRAME_BYTES)
        .await
        .unwrap();
    assert_eq!(response_bytes.as_ref(), decode_hex(response_hex));

    let response_frame = ClientFrame::decode(&response_bytes, MAX_FRAME_BYTES).unwrap();
    assert_eq!(response_frame.protocol_version(), wire_version);
    let ClientWireMessage::Response(response) = response_frame.decode_message().unwrap() else {
        panic!("expected response fixture");
    };
    assert_eq!(response.protocol_version, wire_version);
    assert!(matches!(response.result, Ok(ClientResponse::Evicted)));
}

#[tokio::test]
async fn published_v2_frames_roundtrip_against_current_server() {
    assert_published_fixture_roundtrip(2, V2_REQUEST, V2_RESPONSE).await;
}

#[tokio::test]
async fn frozen_v3_frames_roundtrip_against_current_server() {
    assert_published_fixture_roundtrip(3, V3_REQUEST, V3_RESPONSE).await;
}

#[test]
fn compatibility_fixture_provenance_and_digests_are_frozen() {
    let manifest: Value = serde_json::from_str(PROVENANCE).expect("fixture provenance JSON");
    let manifest_object = manifest.as_object().expect("provenance object");
    assert_eq!(manifest_object.len(), 3, "unexpected provenance fields");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["codec"], FIXTURE_CODEC);
    let fixtures = manifest["fixtures"].as_array().expect("fixtures array");
    assert_eq!(fixtures.len(), FROZEN_FIXTURES.len());

    for (fixture, expected) in fixtures.iter().zip(FROZEN_FIXTURES) {
        let fixture_object = fixture.as_object().expect("fixture object");
        assert_eq!(fixture_object.len(), 8, "unexpected fixture fields");
        assert_eq!(fixture["wire_version"], expected.wire_version);
        assert_eq!(fixture["source_ref"], expected.source_ref);
        assert_eq!(fixture["source_commit"], expected.source_commit);
        assert_eq!(fixture["generated_from"], FIXTURE_GENERATOR);
        assert_eq!(fixture["request_file"], expected.request_file);
        assert_eq!(fixture["request_sha256"], expected.request_sha256);
        assert_eq!(fixture["response_file"], expected.response_file);
        assert_eq!(fixture["response_sha256"], expected.response_sha256);

        // The immutable expectations live in this Rust test, independently of
        // the mutable JSON and hex files. Editing both fixture artifacts cannot
        // bless new bytes or provenance without an explicit code-review diff.
        assert_eq!(
            sha256(&decode_hex(expected.request_hex)),
            expected.request_sha256
        );
        assert_eq!(
            sha256(&decode_hex(expected.response_hex)),
            expected.response_sha256
        );
    }
}

#[tokio::test]
async fn frame_and_envelope_protocol_versions_must_match_before_mutation() {
    let envelope = ClientRequestEnvelope {
        protocol_version: 3,
        ..ClientRequestEnvelope::new(
            "version-mismatch",
            ClientRequest::Get {
                ns: Namespace::new("legacy-v3").unwrap(),
                key: StructuredKey::new(vec!["key".to_owned()]).unwrap(),
            },
        )
    };
    let mut bytes =
        ClientFrame::from_message_with_version(3, &ClientWireMessage::Request(envelope))
            .unwrap()
            .encode()
            .unwrap()
            .to_vec();
    bytes[4..6].copy_from_slice(&2_u16.to_be_bytes());

    let frame = ClientFrame::decode(&bytes, MAX_FRAME_BYTES).unwrap();
    assert_eq!(
        frame.decode_message(),
        Err(ClientProtocolError::VersionMismatch {
            frame_version: 2,
            message_version: 3,
        })
    );

    let surface = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
    let response = surface
        .routes()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(CLIENT_DATA_PATH)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "legacy-client")
                .header(HYDRACACHE_TENANT_HEADER, "legacy-tenant")
                .body(Body::from(bytes))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(surface.state().state_mutations(), 0);
}

#[test]
fn protocol_v2_v3_clients_never_receive_v4_conditional_shapes() {
    for version in [2, 3] {
        let request = ClientRequestEnvelope {
            protocol_version: version,
            ..ClientRequestEnvelope::new(
                format!("conditional-v{version}"),
                ClientRequest::ConditionalPut {
                    ns: Namespace::new(format!("legacy-v{version}")).unwrap(),
                    key: StructuredKey::new(vec!["key".to_owned()]).unwrap(),
                    value: b"value".to_vec(),
                    ttl_ms: None,
                    condition: ConditionalPutCondition::IfAbsent,
                },
            )
        };
        assert_eq!(
            ClientFrame::from_message_with_version(version, &ClientWireMessage::Request(request)),
            Err(ClientProtocolError::UnsupportedMessageForVersion {
                operation: "conditional_put",
                version,
            })
        );

        for response in [
            (
                ClientResponse::ConditionalStored { stored: true },
                "conditional_stored",
            ),
            (
                ClientResponse::CompareValueApplied { applied: true },
                "compare_value_applied",
            ),
        ] {
            let envelope = hydracache_client_protocol::ClientResponseEnvelope::ok(
                format!("conditional-v{version}"),
                response.0,
            )
            .with_protocol_version(version);
            assert_eq!(
                ClientFrame::from_message_with_version(
                    version,
                    &ClientWireMessage::Response(envelope)
                ),
                Err(ClientProtocolError::UnsupportedMessageForVersion {
                    operation: response.1,
                    version,
                })
            );
        }
    }
}
