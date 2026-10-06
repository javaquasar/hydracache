//! Experimental W9b output ownership, not response batching.
//!
//! A value exists only inside one received-read iteration. The first GET is
//! canonical; the second allocates scratch; later equal-sized GETs can reuse it.
//! No next command is decoded/executed on behalf of the writer.

use super::{
    extend_encode_resp2, extend_encode_resp3, resp_value_to_resp2_frame, resp_value_to_resp3_frame,
    BytesMut, RedisCompatError, RespDialect, RespValue,
};

const MIN_PAYLOAD_BYTES: usize = 4096;
const MAX_PAYLOAD_BYTES: usize = 1048576;
const MAX_SCRATCH_BYTES: usize = MAX_PAYLOAD_BYTES + 12;

#[derive(Default)]
pub(super) struct SerialGetScratch {
    previous_payload_bytes: Option<usize>,
    encoded: BytesMut,
    ready: bool,
}

impl SerialGetScratch {
    pub(super) fn reset(&mut self) {
        self.previous_payload_bytes = None;
        self.encoded = BytesMut::new();
        self.ready = false;
    }

    /// Inspect only the already executed actual response; never predict a reply.
    pub(super) fn prepare(&mut self, response: &RespValue, is_get: bool) -> bool {
        let size = match response {
            RespValue::BulkString(value)
                if is_get && (MIN_PAYLOAD_BYTES..=MAX_PAYLOAD_BYTES).contains(&value.len()) =>
            {
                Some(value.len())
            }
            _ => None,
        };
        self.ready = size.is_some() && size == self.previous_payload_bytes;
        if !self.ready {
            // A class/command/error boundary ends the prior reuse episode. Do
            // not retain the largest-ever reply or shrink on each equal GET.
            self.encoded = BytesMut::new();
        }
        self.previous_payload_bytes = size;
        self.ready
    }

    pub(super) fn encode(
        &mut self,
        response: RespValue,
        dialect: RespDialect,
    ) -> Result<&[u8], RedisCompatError> {
        if !self.ready
            || !matches!(&response, RespValue::BulkString(value)
                if Some(value.len()) == self.previous_payload_bytes)
        {
            self.reset();
            return Err(RedisCompatError::Encode(
                "invalid serial GET scratch response".to_owned(),
            ));
        }
        self.ready = false;
        self.encoded.clear();
        let result = match dialect {
            RespDialect::Resp2 => extend_encode_resp2(
                &mut self.encoded,
                &resp_value_to_resp2_frame(response),
                false,
            ),
            RespDialect::Resp3 => extend_encode_resp3(
                &mut self.encoded,
                &resp_value_to_resp3_frame(response),
                false,
            ),
        };
        if let Err(error) = result {
            self.reset();
            return Err(RedisCompatError::Encode(error.to_string()));
        }
        // With the pinned codec/BytesMut implementation, one empty allocation
        // has exactly the required frame capacity and equal sizes never grow it.
        // Guard a future dependency departure loudly, not with an unbounded pool.
        if self.encoded.capacity() > MAX_SCRATCH_BYTES {
            self.reset();
            return Err(RedisCompatError::Encode(
                "serial GET scratch capacity bound exceeded".to_owned(),
            ));
        }
        Ok(&self.encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn first_get_and_out_of_range_responses_never_allocate_scratch() {
        for size in [0, 256, 4095, 4096, 1048576, 1048577] {
            let mut scratch = SerialGetScratch::default();
            let response = RespValue::BulkString(vec![0; size]);
            assert!(!scratch.prepare(&response, true));
            assert_eq!(scratch.encoded.capacity(), 0);
            assert_eq!(
                scratch.prepare(&response, true),
                (4096..=1048576).contains(&size)
            );
            assert_eq!(scratch.encoded.capacity(), 0);
        }
    }

    #[test]
    fn equal_size_replies_reuse_capacity_and_overwrite_exact_resp2_and_resp3_bytes() {
        for size in [4096, 4500, 1048576] {
            for dialect in [RespDialect::Resp2, RespDialect::Resp3] {
                let mut scratch = SerialGetScratch::default();
                assert!(!scratch.prepare(&RespValue::BulkString(vec![0; size]), true));
                let mut pointer = None;
                for byte in [0, 0xff, b'\r', b'\n'] {
                    let response = RespValue::BulkString(vec![byte; size]);
                    let expected =
                        super::super::encode_resp_value(response.clone(), dialect).unwrap();
                    assert!(scratch.prepare(&response, true));
                    let actual = scratch.encode(response, dialect).unwrap();
                    assert!(actual == expected);
                    if let Some(pointer) = pointer {
                        assert_eq!(actual.as_ptr(), pointer);
                    }
                    pointer = Some(actual.as_ptr());
                    assert!(scratch.encoded.capacity() <= MAX_SCRATCH_BYTES);
                }
            }
        }
    }

    #[test]
    fn size_error_missing_and_principal_command_boundaries_release_capacity() {
        for (response, is_get) in [
            (RespValue::BulkString(vec![0; 8192]), true),
            (RespValue::BulkString(vec![0; 4096]), false),
            (RespValue::BulkString(vec![0; 256]), true),
            (RespValue::Error("NOAUTH".to_owned()), true),
            (RespValue::Null, true),
            (RespValue::SimpleString("OK"), false),
        ] {
            let mut scratch = SerialGetScratch::default();
            let large = RespValue::BulkString(vec![0xff; 1048576]);
            assert!(!scratch.prepare(&large, true));
            assert!(scratch.prepare(&large, true));
            scratch.encode(large, RespDialect::Resp2).unwrap();
            assert!(scratch.encoded.capacity() > 0);
            assert!(!scratch.prepare(&response, is_get));
            assert_eq!(scratch.encoded.capacity(), 0);
            scratch.reset();
            assert_eq!(scratch.previous_payload_bytes, None);
        }
    }

    #[test]
    fn misuse_fails_loudly_without_retaining_stale_bytes() {
        let mut scratch = SerialGetScratch::default();
        let response = RespValue::BulkString(vec![0xff; 4096]);
        assert!(!scratch.prepare(&response, true));
        assert!(scratch
            .encode(response.clone(), RespDialect::Resp2)
            .is_err());
        assert!(!scratch.prepare(&response, true));
        assert!(scratch.prepare(&response, true));
        scratch
            .encode(response.clone(), RespDialect::Resp2)
            .unwrap();
        assert!(scratch.prepare(&response, true));
        assert!(scratch.encode(RespValue::Null, RespDialect::Resp2).is_err());
        assert_eq!(scratch.encoded.capacity(), 0);
        assert_eq!(scratch.previous_payload_bytes, None);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]
        #[test]
        fn binary_get_scratch_matches_canonical_codec(
            size in 4096usize..16384,
            bytes in prop::collection::vec(any::<u8>(), 1..128),
            resp3 in any::<bool>(),
        ) {
            let dialect = if resp3 { RespDialect::Resp3 } else { RespDialect::Resp2 };
            let payload: Vec<_> = bytes.iter().copied().cycle().take(size).collect();
            let response = RespValue::BulkString(payload);
            let expected = super::super::encode_resp_value(response.clone(), dialect).unwrap();
            let mut scratch = SerialGetScratch::default();
            prop_assert!(!scratch.prepare(&response, true));
            for _ in 0..3 {
                prop_assert!(scratch.prepare(&response, true));
                prop_assert_eq!(scratch.encode(response.clone(), dialect).unwrap(), expected.as_slice());
                prop_assert!(scratch.encoded.capacity() <= MAX_SCRATCH_BYTES);
            }
        }
    }
}
