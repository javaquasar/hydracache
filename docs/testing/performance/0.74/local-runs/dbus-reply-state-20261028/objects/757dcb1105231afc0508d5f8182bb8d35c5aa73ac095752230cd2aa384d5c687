use zbus::wire::{BE, LE};

#[test]
fn i32_value() {
    let encoded = basic_type_test!(BE, -0x0ABB_AAB0_i32, 4, i32, 4, I32, 8);
    assert_eq!(LE.read_i32(&encoded), 0x5055_44F5_i32);
}
