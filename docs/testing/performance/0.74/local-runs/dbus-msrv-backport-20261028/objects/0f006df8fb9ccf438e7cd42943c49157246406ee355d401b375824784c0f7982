use zbus::wire::BE;

#[test]
fn u32_value() {
    let encoded = basic_type_test!(BE, 0xABBA_ABBA_u32, 4, u32, 4, U32, 8);
    assert_eq!(encoded.len(), 4);
}
