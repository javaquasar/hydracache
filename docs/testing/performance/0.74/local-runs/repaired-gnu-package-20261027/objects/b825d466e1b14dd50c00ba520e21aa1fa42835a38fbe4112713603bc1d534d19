//! Negative D3 is a terminal release decision, never a relaxed acceptance gate.
use std::path::PathBuf;

use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn get_response_owner_full_d3_terminal_rejection_disables_only_candidate() {
    let registry: toml::Value = toml::from_str(
        &std::fs::read_to_string(
            root().join("docs/testing/performance/0.74/proposal-registry.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    let proposal = registry["followup_proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"].as_str() == Some("p74-get-response-owner-v1"))
        .unwrap();
    assert_eq!(
        proposal["state"].as_str(),
        Some("rejected-full-d3-mtls-allocation-floor-runtime-disabled")
    );
    for flag in [
        "runtime_present",
        "default_enabled",
        "accepted_product_change",
        "promotable",
    ] {
        assert_eq!(proposal[flag].as_bool(), Some(false), "{flag}");
    }
    assert_eq!(proposal["full_d3_terminal_rejection"].as_bool(), Some(true));
    assert_eq!(
        proposal["full_d3_allocation_attempts"].as_integer(),
        Some(240)
    );
    let source = std::fs::read_to_string(root().join("crates/hydracache-redis-compat/src/lib.rs"))
        .unwrap()
        .replace("\r\n", "\n");
    let test_module = "#[cfg(test)]\nmod get_response_owner_074;\n\n";
    assert_eq!(source.matches(test_module).count(), 1);
    assert!(!source.contains("get_response_owner_074::reduce"));
    assert!(!source.contains("experimental-resp-get-owner-074"));
    // Same canonical runtime digest as before both the owner and rejected
    // scratch experiments; only historical test helpers remain outside runtime.
    let canonical = source.replace(test_module, "");
    assert_eq!(
        Sha256::digest(canonical.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "c98ab563d4fc9181735d432484118561ad9bf773d32fb2865f4a591dcc50e932"
    );
    let contract: toml::Value = toml::from_str(
        &std::fs::read_to_string(
            root().join("docs/testing/performance/0.74/get-owner-full-d3-allocation-contract.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        contract["matrix"]["maximum_affected_gross_allocation_ratio"].as_float(),
        Some(0.80)
    );
    assert_eq!(
        contract["matrix"]["maximum_native_gross_allocation_ratio"].as_float(),
        Some(1.05)
    );
    assert_eq!(
        contract["matrix"]["maximum_aa_gross_allocation_relative_noise"].as_float(),
        Some(0.01)
    );
    assert_eq!(
        contract["matrix"]["independent_ab_pairs_per_cell"].as_integer(),
        Some(5)
    );
}
