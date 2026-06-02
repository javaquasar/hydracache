#[test]
fn derive_macro_compile_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/derive/pass_entity.rs");
    tests.pass("tests/derive/pass_no_collection.rs");
    tests.compile_fail("tests/derive/fail_missing_entity.rs");
    tests.compile_fail("tests/derive/fail_missing_id.rs");
    tests.compile_fail("tests/derive/fail_unknown_option.rs");
}
