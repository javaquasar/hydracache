// Private projection. Public Serializable remains byte-for-byte unchanged.
pub(crate) trait BorrowedItem: Serializable {
    fn visit(&self, sink: &mut impl FnMut(&[u8]));
}

impl BorrowedItem for str {
    fn visit(&self, sink: &mut impl FnMut(&[u8])) {
        sink(&(self.len() as u16).to_le_bytes());
        sink(self.as_bytes());
    }
}

impl BorrowedItem for ChitchatId {
    fn visit(&self, sink: &mut impl FnMut(&[u8])) {
        self.node_id.as_ref().visit(sink);
        sink(&self.generation_id.to_le_bytes());
        match self.gossip_advertise_addr.ip() {
            IpAddr::V4(ip) => {
                sink(&[4]);
                sink(&ip.octets());
            }
            IpAddr::V6(ip) => {
                sink(&[6]);
                sink(&ip.octets());
            }
        }
        sink(&self.gossip_advertise_addr.port().to_le_bytes());
    }
}

impl CompressedStreamWriter {
    fn legacy_bound(&self, n: usize) -> usize {
        let total = self
            .uncompressed_block
            .len()
            .checked_add(n)
            .expect("item length overflow");
        self.output
            .len()
            .checked_add(total)
            .and_then(|v| v.checked_add(if total > self.block_threshold { 7 } else { 4 }))
            .expect("stream length overflow")
    }

    pub(crate) fn try_append_borrowed<I: BorrowedItem + ?Sized>(
        &mut self,
        item: &I,
        budget: usize,
    ) -> bool {
        let n = item.serialized_len();
        assert!(n > 0);
        if self.legacy_bound(n) > budget {
            return false;
        }
        assert!(n <= u16::MAX as usize);
        let t = self.block_threshold;
        assert!(t > 0);
        let total = self.uncompressed_block.len() + n;
        let encoded_bound = total + 3 * total.div_ceil(t);
        if self
            .output
            .len()
            .checked_add(encoded_bound + 1)
            .is_some_and(|v| v <= budget)
        {
            self.append(item);
            return true;
        }
        let raw_size = t.min(total);
        let needed = raw_size + self.compressed_block.len() + encoded_bound;
        // Non-inlined instantiations keep the maximum frame off small-call stacks.
        match needed {
            0..=512 => self.simulate_borrowed::<512, I>(item, budget, total),
            513..=2048 => self.simulate_borrowed::<2048, I>(item, budget, total),
            2049..=8192 => self.simulate_borrowed::<8192, I>(item, budget, total),
            8193..=32768 => self.simulate_borrowed::<32768, I>(item, budget, total),
            32769..=131072 => self.simulate_borrowed::<131072, I>(item, budget, total),
            131073..=262146 => self.simulate_borrowed::<262146, I>(item, budget, total),
            _ => unreachable!("proved complete u16 item/block storage envelope"),
        }
    }

    #[inline(never)]
    fn simulate_borrowed<const N: usize, I: BorrowedItem + ?Sized>(
        &mut self,
        item: &I,
        budget: usize,
        total: usize,
    ) -> bool {
        let mut space = [0u8; N];
        let t = self.block_threshold;
        let (raw, rest) = space.split_at_mut(t.min(total));
        let old_cache = self.tail_cache;
        let (backup, encoded) = rest.split_at_mut(self.compressed_block.len());
        backup.copy_from_slice(&self.compressed_block[..backup.len()]);
        struct Checkpoint<'a, 'b> {
            w: &'a mut CompressedStreamWriter,
            backup: &'b [u8],
            scratch_len: usize,
            cache: Option<usize>,
            committed: bool,
        }
        impl Drop for Checkpoint<'_, '_> {
            fn drop(&mut self) {
                self.w.compression_context = None;
                if !self.committed {
                    self.w.compressed_block.resize(self.scratch_len, 0);
                    self.w.compressed_block[..self.backup.len()].copy_from_slice(self.backup);
                    self.w.tail_cache = self.cache;
                }
            }
        }
        let scratch_len = self.compressed_block.len();
        let mut checkpoint = Checkpoint {
            w: self,
            backup,
            scratch_len,
            cache: old_cache,
            committed: false,
        };
        let w = &mut *checkpoint.w;
        let mut used = 0;
        let mut seen = 0;
        let mut encoded_len = 0;
        // Pending input is read from the original, never appended speculatively.
        // The private visitor only lends scalar headers and existing field slices.
        let mut feed = |mut bytes: &[u8]| {
            assert!(
                bytes.len() <= total - seen,
                "projection longer than declared item"
            );
            while !bytes.is_empty() {
                let take = (raw.len() - used).min(bytes.len());
                assert!(seen + take <= total, "projection longer than declared item");
                raw[used..used + take].copy_from_slice(&bytes[..take]);
                used += take;
                seen += take;
                bytes = &bytes[take..];
                if used == t && seen < total {
                    w.compressed_block.resize(used, 0);
                    let result = compress_block_with_context(
                        &mut w.compression_context,
                        &raw[..used],
                        &mut w.compressed_block,
                    );
                    let (tag, body) = match result {
                        Ok(n) => (1, &w.compressed_block[..n]),
                        Err(_) => (2, &raw[..used]),
                    };
                    let end = encoded_len + 3 + body.len();
                    encoded[encoded_len] = tag;
                    encoded[encoded_len + 1..encoded_len + 3]
                        .copy_from_slice(&(body.len() as u16).to_le_bytes());
                    encoded[encoded_len + 3..end].copy_from_slice(body);
                    encoded_len = end;
                    used = 0;
                }
            }
        };
        feed(&w.uncompressed_block);
        item.visit(&mut feed);
        assert_eq!(seen, total, "projection shorter than declared item");
        assert!(used > 0 && used <= t);
        w.compressed_block.resize(used, 0);
        let compressed = compress_block_with_context(
            &mut w.compression_context,
            &raw[..used],
            &mut w.compressed_block,
        )
        .ok();
        let exact = w.output.len() + encoded_len + 3 + compressed.unwrap_or(used) + 1;
        if exact > budget {
            return false;
        }
        // This is the only mutation of output/pending. Commit the simulation once.
        w.output.extend_from_slice(&encoded[..encoded_len]);
        w.uncompressed_block.clear();
        w.uncompressed_block.extend_from_slice(&raw[..used]);
        w.tail_cache = compressed;
        checkpoint.committed = true;
        true
    }
}
