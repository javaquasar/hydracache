//! External checks compiled against the retained H17 Rust crate artifact.

#[cfg(test)]
mod tests {
    use hydracache_client_hc2::wire::ClientEnvelope;
    use prost::Message;

    #[test]
    fn retained_decoder_accepts_an_additive_unknown_field() {
        let mut future_envelope = ClientEnvelope::default().encode_to_vec();
        // Field 63, wire type 2, payload "new".
        future_envelope.extend_from_slice(&[0xfa, 0x03, 0x03, b'n', b'e', b'w']);

        let decoded = ClientEnvelope::decode(future_envelope.as_slice())
            .expect("retained decoder must accept a future additive field");
        assert!(decoded.message.is_none());
        assert_eq!(decoded.generation, 0);
    }
}
