use zbus::wire::BE;

#[test]
fn u16_value() {
    let encoded = basic_type_test!(BE, 0xABBA_u16, 2, u16, 2, U16, 6);
    assert_eq!(encoded.len(), 2);
}
