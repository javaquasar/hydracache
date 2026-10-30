//! First resource checkpoint: same immediate-exact-tail input, retained finish Vec.
use super::*;
struct Raw(Vec<u8>);
impl Serializable for Raw {
    fn serialize(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
    fn serialized_len(&self) -> usize {
        self.0.len()
    }
}
impl BorrowedItem for Raw {
    fn visit(&self, sink: &mut impl FnMut(&[u8])) {
        sink(&self.0);
    }
}
#[test]
fn retained_finish_output() {
    let mut seed = 0x0741028_u64;
    let mut bytes: Vec<_> = (0..128)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect();
    bytes.extend_from_slice(&[b'r'; 32]);
    let item = Raw(bytes);
    let mut records = Vec::new();
    for _ in 0..2 {
        let mut w = CompressedStreamWriter::with_block_threshold(64);
        assert!(w.try_append_borrowed(&item, 167));
        let before = [
            w.output.capacity(),
            w.uncompressed_block.capacity(),
            w.compressed_block.capacity(),
        ];
        let output = w.finish();
        let retained = output.capacity();
        let len = output.len();
        let wire: String = output.iter().map(|b| format!("{b:02x}")).collect();
        records.push(serde_json::json!({"append_capacity":before,"finish_capacity":retained,"finish_len":len,"wire":wire}));
    }
    assert_eq!(records[0], records[1]);
    let payload: String = item.0.iter().map(|b| format!("{b:02x}")).collect();
    println!(
        "HC074_RETAINED_FINISH:{}",
        serde_json::json!({"variant":RETENTION_VARIANT,
        "seed":0x0741028_u64,"name":"immediate-exact-tail","threshold":64,"budget":167,"payload":payload,
        "records":records,"scope":"returned Vec capacity only","CPU_qualified":false,"RSS_qualified":false})
    );
}
