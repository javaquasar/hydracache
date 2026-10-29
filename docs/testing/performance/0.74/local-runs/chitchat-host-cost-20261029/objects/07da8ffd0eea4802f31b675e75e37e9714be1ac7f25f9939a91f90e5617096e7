// P: exact ambiguous-path count using this writer's existing pending/scratch.
impl CompressedStreamWriter {
    fn counted_upperbound_after<S: Serializable + ?Sized>(&mut self, item: &S) -> usize {
        let n = item.serialized_len();
        assert!(n > 0);
        let total = self.uncompressed_block.len() + n;
        let old = self.output.len() + total + if total > self.block_threshold { 6 } else { 3 } + 1;
        if n > u16::MAX as usize || total <= self.block_threshold * 2 {
            return old;
        }
        assert!(self.block_threshold > 0, "positive block threshold");
        #[cfg(test)]
        inplace_review::record(1);

        // Internal serializers append stable owned data. Preserve the original
        // pending prefix and both lengths even if serialization unwinds. Scratch
        // contents are disposable; retained pending capacity is explicitly tested.
        struct RestoreLen<'a> {
            bytes: &'a mut Vec<u8>,
            len: usize,
        }
        impl Drop for RestoreLen<'_> {
            fn drop(&mut self) {
                // Capacity already covers the original length, including when
                // scratch was shortened for the final partial block.
                self.bytes.resize(self.len, 0);
            }
        }
        let pending_len = self.uncompressed_block.len();
        let scratch_len = self.compressed_block.len();
        let pending = RestoreLen {
            bytes: &mut self.uncompressed_block,
            len: pending_len,
        };
        let scratch = RestoreLen {
            bytes: &mut self.compressed_block,
            len: scratch_len,
        };
        item.serialize(&mut *pending.bytes);
        let mut exact = self.output.len() + 1; // Original output plus end tag.
        for chunk in pending.bytes.chunks(self.block_threshold) {
            scratch.bytes.resize(chunk.len(), 0);
            let size = zstd::bulk::compress_to_buffer(chunk, &mut scratch.bytes[..], 0)
                .unwrap_or(chunk.len());
            exact += 3 + size; // Same compressed-or-raw choice as flush_block.
        }
        old.max(exact)
    }
}
