use zbus::wire::{LE, serialized::Context, to_bytes};

#[test]
fn struct_ref() {
    let ctxt = Context::new(LE, 0);
    let encoded = to_bytes(ctxt, &(&1u32, &2u32)).unwrap();
    let decoded: [u32; 2] = encoded.deserialize().unwrap().0;
    assert_eq!(decoded, [1u32, 2u32]);
}
