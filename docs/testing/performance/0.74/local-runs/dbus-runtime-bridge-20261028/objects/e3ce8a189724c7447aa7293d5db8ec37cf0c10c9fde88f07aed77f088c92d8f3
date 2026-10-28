use zbus::wire::LE;

#[test]
fn u8_value() {
    let encoded = basic_type_test!(LE, 77_u8, 1, u8, 1, U8, 4);
    assert_eq!(encoded.len(), 1);
}
