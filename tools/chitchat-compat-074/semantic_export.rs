// Appended to the exact caller module; export semantics with the meter inactive.
#[test]
fn compatibility_semantic_export() {
    assert!(!ACTIVE.with(Cell::get));
    let mut rows = Vec::new();
    for case in table().cases {
        let trace = run(&case);
        assert!(!ACTIVE.with(Cell::get));
        rows.push(json!({"input":case,"decisions":&trace.decisions[..trace.attempted],"wire":hex(&trace)}));
    }
    println!(
        "HC074_COMPAT_CALLER:{}",
        json!({"seed":0x0741028_u64,"table_sha256":"4a3308a82d014fb58df808a9f3984922537a80c02219610cdf5ee6fa1ec4a1f9",
        "variant":CALLER_VARIANT,"rows":rows,"observer_active":false,"native_C_qualified":false,"CPU_qualified":false,"resources_qualified":false})
    );
}
