use super::*;
use hydracache_cluster_testkit::current_daemon_wire_075::ClientResponseEnvelope;

fn replies(step: usize, wrong: bool) -> Vec<u8> {
    let first = (77u64 ^ 9).to_be_bytes();
    let second = (77u64 ^ 9 ^ 0x8000_0000_0000_0000).to_be_bytes();
    if step == 2 {
        let mut result = b"$8\r\n".to_vec();
        result.extend_from_slice(&first);
        result.extend_from_slice(b"\r\n+OK\r\n");
        return if wrong {
            b"$-1\r\n+OK\r\n".to_vec()
        } else {
            result
        };
    }
    if step == 3 {
        return if wrong {
            b"-ERR unit refusal\r\n".to_vec()
        } else {
            b"+OK\r\n+OK\r\n".to_vec()
        };
    }
    let result = match (step, wrong) {
        (1, false) => ClientResponse::Stored,
        (4, false) => ClientResponse::Value {
            value: Some(second.to_vec()),
        },
        (5, false) => ClientResponse::Invalidated,
        (6, false) => ClientResponse::Value { value: None },
        _ => ClientResponse::Value {
            value: Some(vec![1]),
        },
    };
    let body = ClientFrame::from_message(&ClientWireMessage::Response(ClientResponseEnvelope::ok(
        "unit", result,
    )))
    .unwrap()
    .encode()
    .unwrap();
    let mut response =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
    response.extend_from_slice(&body);
    response
}
fn addresses() -> [SocketAddr; 3] {
    [
        "127.0.0.1:1".parse().unwrap(),
        "127.0.0.1:2".parse().unwrap(),
        "127.0.0.1:3".parse().unwrap(),
    ]
}
#[test]
fn each_transport_failure_stops_before_later_protocol_operations() {
    let addrs = addresses();
    for failed in 1..=6 {
        let mut calls = 0;
        let result = soak_round_using(&addrs, 77, 9, |address, request| {
            calls += 1;
            assert_eq!(
                address,
                if calls == 2 || calls == 3 {
                    addrs[2]
                } else {
                    addrs[0]
                }
            );
            assert!(!request.is_empty());
            if calls == failed {
                return Err("unit transport refusal".into());
            }
            Ok(replies(calls, false))
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unit transport refusal"));
        assert_eq!(calls, failed);
    }
}
#[test]
fn every_wrong_hc1_or_resp_result_refuses_and_valid_round_preserves_binary_values() {
    for wrong_at in 0..=6 {
        let mut calls = 0;
        let result = soak_round_using(&addresses(), 77, 9, |_, request| {
            calls += 1;
            if calls == 2 {
                assert!(request.starts_with(b"*2\r\n$3\r\nGET"));
            }
            if calls == 3 {
                assert!(request
                    .windows(8)
                    .any(|w| w == (77u64 ^ 9 ^ 0x8000_0000_0000_0000).to_be_bytes()));
            }
            Ok(replies(calls, calls == wrong_at))
        });
        if wrong_at == 0 {
            result.unwrap();
            assert_eq!(calls, 6);
        } else {
            assert!(result.is_err());
            assert_eq!(calls, wrong_at);
        }
    }
}
#[test]
fn malformed_http_status_and_frame_refuse_at_each_hc1_boundary() {
    for boundary in [1, 4, 5, 6] {
        for malformed in [
            b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n".as_slice(),
            b"broken",
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx",
        ] {
            let mut calls = 0;
            assert!(soak_round_using(&addresses(), 77, 9, |_, _| {
                calls += 1;
                Ok(if calls == boundary {
                    malformed.to_vec()
                } else {
                    replies(calls, false)
                })
            })
            .is_err());
            assert_eq!(calls, boundary);
        }
    }
}
