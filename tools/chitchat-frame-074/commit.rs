// Only this commit schedule differs from F. Input is the internal simulation's
// complete encoded prefix; it is not an external transport/decoder surface.
impl CompressedStreamWriter {
    fn commit_simulated_prefix(&mut self, mut encoded: &[u8]) {
        while !encoded.is_empty() {
            let header: [u8; 3] = encoded[..3].try_into().unwrap();
            assert!(matches!(header[0], 1 | 2), "internal block tag");
            let length = u16::from_le_bytes([header[1], header[2]]);
            let (frame, remaining) = encoded.split_at(3 + usize::from(length));
            // Exactly the original flush_block Vec operations: tag, LE length,
            // then body. No reservation or bulk extension of several frames.
            header[0].serialize(&mut self.output);
            length.serialize(&mut self.output);
            self.output.extend(&frame[3..]);
            encoded = remaining;
        }
    }
}
