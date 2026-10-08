//! Portable test target; Linux manager checks run in the ordinary Linux CI lane.
#[cfg(target_os = "linux")]
mod linux {

    use hydracache_long_run_supervisor_074::diagnostic_manager::ManagerScope;

    #[test]
    fn manager_scope_has_only_fixed_diagnostic_units() {
        let lease = "a".repeat(64);
        let boot = "12345678-1234-1234-1234-123456789abc";
        for (index, surface) in ["embedded", "direct", "resp2", "resp3"].iter().enumerate() {
            let scope = ManagerScope::new(&lease, boot, surface).unwrap();
            assert_eq!(
                scope.unit_name(),
                format!("hydracache-diagnostic-074-{lease}-{}.service", index + 1)
            );
        }
        for bad in ["", "../resp2", "native", "RESP2", "resp2\n"] {
            assert!(ManagerScope::new(&lease, boot, bad).is_err());
        }
        assert!(ManagerScope::new(&"A".repeat(64), boot, "resp2").is_err());
        assert!(ManagerScope::new(&lease, "../boot", "resp2").is_err());
    }

    #[test]
    fn scope_wire_refuses_future_unknown_duplicate_and_noncanonical_input() {
        let scope = ManagerScope::new(
            &"a".repeat(64),
            "12345678-1234-1234-1234-123456789abc",
            "resp2",
        )
        .unwrap();
        let bytes = scope.encode().unwrap();
        assert_eq!(ManagerScope::decode(&bytes).unwrap(), scope);
        let text = String::from_utf8(bytes.clone()).unwrap();
        for bad in [
            text.replace("\"schema_version\":1", "\"schema_version\":2"),
            text.replacen('{', "{\"argv\":[],", 1),
            text.replacen('{', "{\"surface\":\"resp2\",", 1),
            format!("{text}\n"),
        ] {
            assert!(ManagerScope::decode(bad.as_bytes()).is_err(), "{bad}");
        }
        assert!(ManagerScope::decode(&vec![b' '; 4097]).is_err());
    }
}
