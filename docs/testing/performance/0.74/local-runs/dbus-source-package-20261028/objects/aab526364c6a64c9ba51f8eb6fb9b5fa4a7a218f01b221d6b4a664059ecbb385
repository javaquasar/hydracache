use zbus::wire::{BE, LE};

#[test]
fn i16_value() {
    let encoded = basic_type_test!(BE, -0xAB0_i16, 2, i16, 2, I16, 6);
    assert_eq!(LE.read_i16(&encoded), 0x50F5_i16);
}
