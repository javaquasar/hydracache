use hydracache_long_run_supervisor_074::protocol::{
    parse_request, parse_wire_request, sign_response, verify_response, ProtocolError, ResponseBody,
    MAX_PACKET_BYTES,
};
use serde_json::json;

fn request(operation: &str) -> serde_json::Value {
    let campaign = "a".repeat(64);
    json!({
        "schema_version": 1,
        "request_id": "123e4567-e89b-42d3-a456-426614174000",
        "operation": operation,
        "campaign_id": campaign,
        "expected_state_revision": 7,
        "manifest_path": if operation == "start" {
            Some(format!("/var/lib/hydracache-performance/staging/{}/campaign-start.json", "a".repeat(64)))
        } else { None },
        "manifest_sha256": "b".repeat(64),
        "controller": {
            "repository_id": 1, "run_id": 2, "run_attempt": 1, "actor_id": 3,
            "authorization_sha256": "c".repeat(64)
        },
        "abort_reason": if operation == "abort" { Some("operator-request") } else { None },
        "approval_nonce_sha256": if operation == "abort" { Some("d".repeat(64)) } else { None }
    })
}

#[test]
fn wire_envelope_requires_authorization_only_for_mutating_operations() {
    let status = json!({"request": request("status"), "authorization": null});
    assert!(parse_wire_request(&serde_json::to_vec(&status).unwrap()).is_ok());

    let attach_without_authorization = json!({"request": request("attach"), "authorization": null});
    assert_eq!(
        parse_wire_request(&serde_json::to_vec(&attach_without_authorization).unwrap()),
        Err(ProtocolError::OperationFields)
    );

    let unexpected = json!({
        "request": request("status"),
        "authorization": {
            "body": {
                "schema_version": 1,
                "request_id": "123e4567-e89b-42d3-a456-426614174000",
                "operation": "status",
                "campaign_id": "a".repeat(64),
                "manifest_sha256": "b".repeat(64),
                "repository_id": 1,
                "run_id": 2,
                "actor_id": 3,
                "issued_at_unix_seconds": 1,
                "expires_at_unix_seconds": 2
            },
            "signature_hex": "c".repeat(128)
        }
    });
    assert_eq!(
        parse_wire_request(&serde_json::to_vec(&unexpected).unwrap()),
        Err(ProtocolError::OperationFields)
    );
}

#[test]
fn strict_protocol_accepts_allowlisted_operations() {
    for operation in ["start", "attach", "status", "seal", "abort", "verify"] {
        let bytes = serde_json::to_vec(&request(operation)).unwrap();
        assert_eq!(parse_request(&bytes).unwrap().schema_version, 1);
    }
}

#[test]
fn rejects_oversized_truncated_unknown_duplicate_and_invalid_utf8_packets() {
    assert_eq!(
        parse_request(&vec![b' '; MAX_PACKET_BYTES + 1]),
        Err(ProtocolError::Size)
    );
    assert!(matches!(
        parse_request(br#"{"schema_version":1"#),
        Err(ProtocolError::Json(_))
    ));

    let mut unknown = request("status");
    unknown["shell"] = json!("sh -c id");
    assert!(matches!(
        parse_request(&serde_json::to_vec(&unknown).unwrap()),
        Err(ProtocolError::Json(_))
    ));

    let valid = serde_json::to_string(&request("status")).unwrap();
    let duplicate = valid.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(matches!(
        parse_request(duplicate.as_bytes()),
        Err(ProtocolError::Json(_))
    ));
    assert!(matches!(
        parse_request(&[0xff]),
        Err(ProtocolError::Json(_))
    ));
}

#[test]
fn rejects_path_traversal_and_cross_operation_fields() {
    let mut traversal = request("start");
    traversal["manifest_path"] = json!(format!(
        "/var/lib/hydracache-performance/staging/{}/../campaign-start.json",
        "a".repeat(64)
    ));
    assert_eq!(
        parse_request(&serde_json::to_vec(&traversal).unwrap()),
        Err(ProtocolError::ManifestPath)
    );

    let mut status = request("status");
    status["manifest_path"] = json!("/tmp/untrusted.json");
    assert_eq!(
        parse_request(&serde_json::to_vec(&status).unwrap()),
        Err(ProtocolError::OperationFields)
    );
}

#[test]
fn response_digest_excludes_only_its_own_field() {
    let body = ResponseBody {
        schema_version: 1,
        request_id: "123e4567-e89b-42d3-a456-426614174000".to_owned(),
        campaign_id: "a".repeat(64),
        ok: true,
        state_revision: 8,
        server_time_unix_seconds: 1_000,
        result: Some(json!({"state": "I74_RUNNING"})),
        error_code: None,
    };
    let mut response = sign_response(body).unwrap();
    assert!(verify_response(&response).is_ok());
    response.body.state_revision += 1;
    assert_eq!(verify_response(&response), Err(ProtocolError::Identity));
}
