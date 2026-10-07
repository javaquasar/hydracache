//! Off-by-default D2 experiment. Only private, length-sized successful GET
//! responses transfer ownership; all other cases use the public borrowed oracle.
use super::{
    ClientRequest, ClientResponse, ClientResponseEnvelope, RedisExecutionPlan, RedisFollowup,
    RedisResponseReducer, RedisTranslationError, RespValue,
};

pub(super) fn reduce(
    plan: &RedisExecutionPlan,
    mut responses: Vec<ClientResponseEnvelope>,
) -> Result<RespValue, RedisTranslationError> {
    if !matches!(&plan.reducer, RedisResponseReducer::Get)
        || !matches!(&plan.followup, RedisFollowup::None)
        || !matches!(plan.initial_requests(), [request] if matches!(&request.request, ClientRequest::Get { .. }))
        || responses.len() != 1
    {
        return plan.reduce(&responses);
    }
    let Ok(ClientResponse::Value { value: Some(value) }) = &mut responses[0].result else {
        return plan.reduce(&responses);
    };
    // A larger response owner could outlive the canonical length-sized clone
    // during encoding. Keep that uncommon shape on the canonical path.
    if value.is_empty() || value.capacity() != value.len() {
        return plan.reduce(&responses);
    }
    // Vec::default has no allocation. The envelope retains no payload owner;
    // the result is owned, never pooled or borrowed across the writer await.
    Ok(RespValue::BulkString(std::mem::take(value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encode_resp2_value, encode_resp3_value, translate_redis_command, ClientErrorCode,
        ClientErrorEnvelope, Namespace, RedisCommand, RedisTranslatedCommand,
        RedisTranslationContext, RedisTtlUnit,
    };
    use proptest::prelude::*;
    use proptest::test_runner::{Config, RngSeed, TestRunner};

    fn get_plan() -> RedisExecutionPlan {
        let command = RedisCommand::Get {
            key: b"binary\0\xff".to_vec(),
        };
        let context = RedisTranslationContext::new("redis", "owner-test").unwrap();
        let RedisTranslatedCommand::Execute(plan) =
            translate_redis_command(command, &context).unwrap()
        else {
            panic!()
        };
        plan
    }
    fn response(value: Option<Vec<u8>>) -> Vec<ClientResponseEnvelope> {
        vec![ClientResponseEnvelope::ok(
            "value-owner",
            ClientResponse::Value { value },
        )]
    }
    fn payload_pointer(responses: &[ClientResponseEnvelope]) -> *const u8 {
        let Ok(ClientResponse::Value { value: Some(value) }) = &responses[0].result else {
            panic!()
        };
        value.as_ptr()
    }

    #[test]
    fn successful_get_transfers_pointer_and_preserves_resp2_resp3_bytes() {
        let plan = get_plan();
        for size in [1, 64, 4096, 1048576] {
            let responses = response(Some((0..size).map(|i| (i % 256) as u8).collect()));
            let pointer = payload_pointer(&responses);
            let expected = plan.reduce(&responses).unwrap();
            let RespValue::BulkString(cloned) = &expected else {
                panic!()
            };
            assert_ne!(
                cloned.as_ptr(),
                pointer,
                "independent borrowed oracle must clone"
            );
            let actual = reduce(&plan, responses).unwrap();
            let RespValue::BulkString(transferred) = &actual else {
                panic!()
            };
            assert_eq!(transferred.as_ptr(), pointer);
            assert_eq!(transferred.capacity(), size);
            assert_eq!(actual, expected);
            assert_eq!(
                encode_resp2_value(actual.clone()).unwrap(),
                encode_resp2_value(expected.clone()).unwrap()
            );
            assert_eq!(
                encode_resp3_value(actual).unwrap(),
                encode_resp3_value(expected).unwrap()
            );
        }
    }

    #[test]
    fn spare_capacity_empty_and_missing_values_use_canonical_fallback() {
        let plan = get_plan();
        let mut payload = Vec::with_capacity(8192);
        payload.extend_from_slice(&[0xff; 4096]);
        assert!(payload.capacity() > payload.len());
        let responses = response(Some(payload));
        let pointer = payload_pointer(&responses);
        let expected = plan.reduce(&responses).unwrap();
        let actual = reduce(&plan, responses).unwrap();
        let RespValue::BulkString(value) = &actual else {
            panic!()
        };
        assert_ne!(value.as_ptr(), pointer);
        assert_eq!(value.capacity(), value.len());
        assert_eq!(actual, expected);
        for value in [Some(Vec::new()), None] {
            let responses = response(value);
            assert_eq!(reduce(&plan, responses.clone()), plan.reduce(&responses));
        }
    }

    #[test]
    fn wrong_counts_kinds_and_every_client_error_preserve_canonical_details() {
        let plan = get_plan();
        for count in [0, 2, 3] {
            let responses = vec![
                ClientResponseEnvelope::ok(
                    "extra",
                    ClientResponse::Value {
                        value: Some(vec![0xff; 64])
                    }
                );
                count
            ];
            assert_eq!(reduce(&plan, responses.clone()), plan.reduce(&responses));
        }
        for kind in [
            ClientResponse::Stored,
            ClientResponse::Invalidated,
            ClientResponse::ConditionalStored { stored: true },
            ClientResponse::Batch { items: Vec::new() },
        ] {
            let responses = vec![ClientResponseEnvelope::ok("wrong-kind", kind)];
            assert_eq!(reduce(&plan, responses.clone()), plan.reduce(&responses));
        }
        for code in [
            ClientErrorCode::IncompatibleVersion,
            ClientErrorCode::Unauthenticated,
            ClientErrorCode::Unauthorized,
            ClientErrorCode::TenantQuota,
            ClientErrorCode::RateLimited,
            ClientErrorCode::ResidencyDenied,
            ClientErrorCode::TooLarge,
            ClientErrorCode::DeadlineExceeded,
            ClientErrorCode::Conflict,
            ClientErrorCode::BackendUnavailable,
            ClientErrorCode::MalformedFrame,
        ] {
            for retryable in [false, true] {
                let responses = vec![ClientResponseEnvelope::error(
                    "error",
                    ClientErrorEnvelope::new(code, retryable, "canonical error"),
                )];
                assert_eq!(reduce(&plan, responses.clone()), plan.reduce(&responses));
            }
        }
    }

    #[test]
    fn invalid_initial_and_followup_plans_do_not_transfer_payload() {
        let original = get_plan();
        let mut empty = original.clone();
        empty.initial_requests.clear();
        let mut extra = original.clone();
        extra
            .initial_requests
            .push(original.initial_requests[0].clone());
        let mut non_get = original.clone();
        non_get.initial_requests[0].request = ClientRequest::Invalidate {
            ns: Namespace::new("redis").unwrap(),
            key: match &original.initial_requests[0].request {
                ClientRequest::Get { key, .. } => key.clone(),
                _ => panic!(),
            },
        };
        let mut followup = original;
        followup.followup = RedisFollowup::InvalidateExisting {
            namespace: Namespace::new("redis").unwrap(),
            keys: Vec::new(),
            request_id: "followup".to_owned(),
        };
        for plan in [empty, extra, non_get, followup] {
            let responses = response(Some(vec![0xff; 64]));
            let pointer = payload_pointer(&responses);
            let expected = plan.reduce(&responses).unwrap();
            let actual = reduce(&plan, responses).unwrap();
            let RespValue::BulkString(value) = &actual else {
                panic!()
            };
            assert_ne!(value.as_ptr(), pointer);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn every_non_get_reducer_delegates_to_the_existing_borrowed_path() {
        for reducer in [
            RedisResponseReducer::Set,
            RedisResponseReducer::ConditionalStoredInteger,
            RedisResponseReducer::Mset,
            RedisResponseReducer::Mget { expected_items: 1 },
            RedisResponseReducer::Del { expected_items: 1 },
            RedisResponseReducer::Exists { expected_items: 1 },
            RedisResponseReducer::Type,
            RedisResponseReducer::Expiry,
            RedisResponseReducer::Ttl {
                unit: RedisTtlUnit::Milliseconds,
            },
            RedisResponseReducer::Invalidate,
            RedisResponseReducer::CompareValueApplied,
        ] {
            let mut plan = get_plan();
            plan.reducer = reducer;
            let responses = response(Some(vec![0xff; 64]));
            assert_eq!(reduce(&plan, responses.clone()), plan.reduce(&responses));
        }
    }

    #[test]
    fn response_metadata_does_not_introduce_a_new_protocol_gate() {
        let plan = get_plan();
        for version in [1, 4, u16::MAX] {
            let mut responses = response(Some(vec![0xff; 64]));
            responses[0].protocol_version = version;
            responses[0].request_id = format!("metadata-{version}");
            let pointer = payload_pointer(&responses);
            let expected = plan.reduce(&responses).unwrap();
            let actual = reduce(&plan, responses).unwrap();
            let RespValue::BulkString(value) = &actual else {
                panic!()
            };
            assert_eq!(value.as_ptr(), pointer);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn seeded_property_matches_independent_borrowed_oracle_and_ownership() {
        let plan = get_plan();
        let mut runner = TestRunner::new(Config {
            cases: 128,
            rng_seed: RngSeed::Fixed(740074),
            failure_persistence: None,
            ..Config::default()
        });
        eprintln!("GET response ownership property seed=740074");
        runner
            .run(
                &(
                    proptest::collection::vec(any::<u8>(), 0..4097),
                    any::<bool>(),
                    any::<bool>(),
                ),
                |(payload, hit, spare_capacity)| {
                    let mut payload = payload.into_boxed_slice().into_vec();
                    if spare_capacity {
                        payload.reserve_exact(1);
                    }
                    let responses = response(hit.then_some(payload));
                    let owner = if let Ok(ClientResponse::Value { value: Some(value) }) =
                        &responses[0].result
                    {
                        (!value.is_empty())
                            .then_some((value.as_ptr(), value.capacity() == value.len()))
                    } else {
                        None
                    };
                    let expected = plan.reduce(&responses).unwrap();
                    let actual = reduce(&plan, responses).unwrap();
                    if let Some((pointer, admitted)) = owner {
                        let RespValue::BulkString(value) = &actual else {
                            return Err(TestCaseError::fail("nonempty result shape differs"));
                        };
                        if admitted {
                            prop_assert_eq!(value.as_ptr(), pointer);
                        } else {
                            prop_assert_ne!(value.as_ptr(), pointer);
                            prop_assert_eq!(value.capacity(), value.len());
                        }
                    }
                    prop_assert_eq!(actual, expected);
                    Ok(())
                },
            )
            .unwrap();
    }
}
