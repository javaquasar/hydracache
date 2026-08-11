use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{client_conformance, client_plane_generation, client_plane_python};

const GENERATION: u64 = 6;
const CONTRACT: &str = "crates/hydracache-client-hc2/proto/hc2_contract.proto";
const API: &str = "docs/compatibility/hc2-sdk-api-v1.json";
const PYTHON_METADATA: &str =
    "sdks/python/hydracache-client-hc2/src/hydracache_hc2_generated/contract_metadata.json";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-schema-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub(crate) fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    validate_contract(root)?;
    validate_generation_constants(root)?;
    validate_api_manifest(root)?;
    validate_python_metadata(root)?;
    client_conformance::validate_manifest_at_root(root)?;
    client_plane_generation::check_at_root(root)?;
    client_plane_python::check_generated_at_root(root)?;
    println!(
        "client-schema-check: OK (generation {GENERATION}, Rust/Java/Python generation is clean and byte-deterministic)"
    );
    Ok(())
}

fn validate_contract(root: &Path) -> Result<(), Box<dyn Error>> {
    let path = root.join(CONTRACT);
    let source = fs::read_to_string(&path)?;
    let required = [
        "package hydracache.client.v2alpha;",
        "service ClientPlaneAlpha",
        "rpc Open(stream ClientEnvelope) returns (stream ServerEnvelope);",
        "CompareAndSetRequest compare_and_set",
        "RemoveIfValueRequest remove_if_value",
        "TryLockRequest try_lock",
        "LockOwnershipRequest lock_ownership",
        "message Subscribe",
        "message SessionOpen",
    ];
    for marker in required {
        if !source.contains(marker) {
            return Err(format!(
                "{} is missing required HC/2 marker {marker:?}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn validate_generation_constants(root: &Path) -> Result<(), Box<dyn Error>> {
    let required = [
        (
            "crates/hydracache-client-hc2/src/lib.rs",
            "pub const HC2_GENERATION: u32 = 6;",
        ),
        (
            "sdks/java/hydracache-client-hc2/src/main/java/io/hydracache/client/hc2/HydraCacheClientConfig.java",
            "CURRENT_PROTOCOL_GENERATION = 6;",
        ),
        (
            "sdks/python/hydracache-client-hc2/src/hydracache_hc2/models.py",
            "protocol_generation: int = 6",
        ),
    ];
    for (relative, marker) in required {
        let path = root.join(relative);
        if !fs::read_to_string(&path)?.contains(marker) {
            return Err(format!(
                "{} does not bind the SDK to protocol generation {GENERATION}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn validate_api_manifest(root: &Path) -> Result<(), Box<dyn Error>> {
    let path = root.join(API);
    let value: Value = serde_json::from_slice(&fs::read(&path)?)?;
    if value.get("protocol_generation").and_then(Value::as_u64) != Some(GENERATION) {
        return Err(format!(
            "{} must freeze protocol_generation {GENERATION}",
            path.display()
        )
        .into());
    }
    let packages = value
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{} packages must be an array", path.display()))?;
    let coordinates = packages
        .iter()
        .filter_map(|package| package.get("coordinate").and_then(Value::as_str))
        .collect::<Vec<_>>();
    for coordinate in [
        "hydracache-client-hc2",
        "io.hydracache:hydracache-client-hc2",
        "io.hydracache:hydracache-hazelcast-facade",
    ] {
        if !coordinates.contains(&coordinate) {
            return Err(format!("{} does not freeze package {coordinate}", path.display()).into());
        }
    }
    Ok(())
}

fn validate_python_metadata(root: &Path) -> Result<(), Box<dyn Error>> {
    let path = root.join(PYTHON_METADATA);
    let value: Value = serde_json::from_slice(&fs::read(&path)?)?;
    let files = value
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{} files must be an array", path.display()))?;
    let contract = files
        .iter()
        .find(|file| file.get("name").and_then(Value::as_str) == Some("hc2_contract.proto"))
        .ok_or_else(|| format!("{} omits hc2_contract.proto", path.display()))?;
    if contract.get("package").and_then(Value::as_str) != Some("hydracache.client.v2alpha") {
        return Err(format!("{} contains the wrong HC/2 package", path.display()).into());
    }
    let services = contract
        .get("services")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{} omits services", path.display()))?;
    let service = services
        .iter()
        .find(|service| service.get("name").and_then(Value::as_str) == Some("ClientPlaneAlpha"));
    if service.is_none() {
        return Err(format!("{} omits ClientPlaneAlpha", path.display()).into());
    }
    Ok(())
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join(CONTRACT).is_file() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_manifests_and_sdk_generations_agree() {
        let root = workspace_root().unwrap();
        validate_contract(&root).unwrap();
        validate_generation_constants(&root).unwrap();
        validate_api_manifest(&root).unwrap();
        validate_python_metadata(&root).unwrap();
    }
}
