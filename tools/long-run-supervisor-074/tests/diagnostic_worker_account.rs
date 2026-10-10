//! Fixed helper integration; never creates an account or starts a product unit.
#[cfg(target_os = "linux")]
mod linux {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const BINARY: &str = env!("CARGO_BIN_EXE_hydracache-long-run-supervisor-074");

    #[test]
    fn real_account_helper_refuses_noncanonical_selectors_and_byte_budget() {
        for request in [
            b"{}".to_vec(),
            b"{\"schema_version\":1,\"user\":\"root\"}".to_vec(),
            b"{\"schema_version\":1}\n".to_vec(),
            vec![b' '; 4097],
        ] {
            let mut child = Command::new(BINARY)
                .arg("diagnostic-worker-account-worker")
                .env_clear()
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(&request).unwrap();
            let output = child.wait_with_output().unwrap();
            assert_eq!(output.status.code(), Some(9));
            assert!(output.stdout.is_empty());
            assert_eq!(
                output.stderr,
                b"diagnostic worker account observation refused\n"
            );
        }
    }
    #[test]
    fn operator_fixed_account_observation_is_explicit_not_enrollment() {
        let output = Command::new(BINARY)
            .arg("diagnostic-worker-account-inspect")
            .env_clear()
            .output()
            .unwrap();
        eprintln!(
            "account operator exit={:?} stdout={} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.len() <= 4097);
        assert!(output.stderr.len() <= 256);
        if output.status.success() {
            assert!(output.stderr.is_empty());
            let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(doc["schema_version"], 1);
            assert_eq!(doc["account"], "hydracache-perf");
            assert_eq!(doc["group"], "hydracache-perf");
            for field in ["uid", "gid"] {
                let id = doc[field].as_u64().unwrap();
                assert!(id > 0 && id < u32::MAX as u64);
            }
            let groups = doc["membership_gids"].as_array().unwrap();
            assert!(!groups.is_empty() && groups.len() <= 32);
            assert!(groups.contains(&doc["gid"]));
        } else {
            assert_eq!(output.status.code(), Some(9));
            assert!(output.stdout.is_empty());
            // Missing account is a refusal, never silently substituted with the caller.
            assert_eq!(
                output.stderr,
                b"worker account observation refused: Exit; helper cleanup=true\n"
            );
        }
    }
    #[test]
    fn operator_rejects_account_or_helper_selectors() {
        for mode in [
            "diagnostic-worker-account-inspect",
            "diagnostic-worker-account-worker",
        ] {
            let output = Command::new(BINARY)
                .args([mode, "root"])
                .env_clear()
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).starts_with("usage:"));
        }
    }
}
