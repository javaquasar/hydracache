use zbus::wire::LE;

#[test]
fn bool_value() {
    let encoded = basic_type_test!(LE, true, 4, bool, 4, Bool, 8);
    assert_eq!(encoded.len(), 4);
}
