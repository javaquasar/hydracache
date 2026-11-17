use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

const RUST_CRATE: &str = "hydracache-client-hc2";
const JAVA_POM: &str = "sdks/java/hydracache-client-hc2/pom.xml";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-generation-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let generation_root = root.join("target").join("hc2-generation-check");
    recreate_owned_directory(root, &generation_root)?;

    let package_id = rust_package_id(root)?;
    let rust_a = generate_rust(root, &generation_root.join("rust-a"), &package_id)?;
    let rust_b = generate_rust(root, &generation_root.join("rust-b"), &package_id)?;
    compare("Rust", &rust_a, &rust_b)?;

    let java_a = generate_java(root)?;
    let java_b = generate_java(root)?;
    compare("Java", &java_a, &java_b)?;

    println!(
        "client-plane-generation-check: OK (Rust {} files, Java {} files; two clean byte-identical generations)",
        rust_a.len(),
        java_a.len()
    );
    Ok(())
}

fn generate_rust(
    root: &Path,
    target_dir: &Path,
    package_id: &str,
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let output = Command::new("cargo")
        .args(["check", "--locked", "-p", RUST_CRATE, "--target-dir"])
        .arg(target_dir)
        .arg("--message-format=json-render-diagnostics")
        .current_dir(root)
        .stderr(std::process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(format!("clean Rust HC/2 generation failed with {}", output.status).into());
    }

    let out = rust_output_dir(std::str::from_utf8(&output.stdout)?, package_id)?.canonicalize()?;
    let target_dir = target_dir.canonicalize()?;
    if out == target_dir || !out.starts_with(&target_dir) {
        return Err(format!(
            "Rust generation output is outside its target: {}",
            out.display()
        )
        .into());
    }
    let mut generated = BTreeMap::new();
    collect_files(&out, &out, &mut generated)?;
    require_non_empty("Rust", generated)
}

fn rust_package_id(root: &Path) -> Result<String, Box<dyn Error>> {
    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
    }

    #[derive(Deserialize)]
    struct Package {
        name: String,
        id: String,
    }

    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--no-deps", "--format-version=1"])
        .current_dir(root)
        .stderr(std::process::Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(format!("Rust HC/2 package metadata failed with {}", output.status).into());
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)?;
    let mut packages = metadata
        .packages
        .into_iter()
        .filter(|package| package.name == RUST_CRATE);
    match (packages.next(), packages.next()) {
        (Some(package), None) => Ok(package.id),
        _ => Err(format!("expected exactly one workspace package named {RUST_CRATE}").into()),
    }
}

fn rust_output_dir(messages: &str, package_id: &str) -> Result<PathBuf, Box<dyn Error>> {
    let mut out = None;
    for line in messages.lines() {
        let message: serde_json::Value = serde_json::from_str(line)?;
        if message["reason"] != "build-script-executed" || message["package_id"] != package_id {
            continue;
        }
        if out.is_some() {
            return Err(format!("multiple Cargo build-script outputs for {package_id}").into());
        }
        out = Some(PathBuf::from(
            message["out_dir"]
                .as_str()
                .ok_or("Cargo build-script output is missing out_dir")?,
        ));
    }
    out.ok_or_else(|| format!("missing Cargo build-script output for {package_id}").into())
}

fn generate_java(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    run_checked(
        root,
        maven_program(),
        &["-B", "-ntp", "-f", JAVA_POM, "clean", "generate-sources"],
        "clean Java HC/2 generation",
    )?;
    let generated_root = root
        .join("sdks")
        .join("java")
        .join("hydracache-client-hc2")
        .join("target")
        .join("generated-sources");
    let mut generated = BTreeMap::new();
    collect_files(&generated_root, &generated_root, &mut generated)?;
    require_non_empty("Java", generated)
}

fn collect_files(
    root: &Path,
    current: &Path,
    output: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, output)?;
        } else if path.is_file() {
            let key = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            output.insert(key, fs::read(path)?);
        }
    }
    Ok(())
}

fn require_non_empty(
    language: &str,
    generated: BTreeMap<String, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    if generated.is_empty() {
        Err(format!("{language} generation produced no files").into())
    } else {
        Ok(generated)
    }
}

fn compare(
    language: &str,
    first: &BTreeMap<String, Vec<u8>>,
    second: &BTreeMap<String, Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    if first == second {
        return Ok(());
    }
    let missing = first
        .keys()
        .filter(|key| !second.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let extra = second
        .keys()
        .filter(|key| !first.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let changed = first
        .iter()
        .filter_map(|(key, bytes)| {
            second
                .get(key)
                .filter(|other| *other != bytes)
                .map(|_| key.clone())
        })
        .collect::<Vec<_>>();
    Err(format!(
        "{language} generation is not byte-identical; missing={missing:?} extra={extra:?} changed={changed:?}"
    )
    .into())
}

fn recreate_owned_directory(root: &Path, directory: &Path) -> Result<(), Box<dyn Error>> {
    let expected = root.join("target").join("hc2-generation-check");
    if directory != expected || !directory.starts_with(root.join("target")) {
        return Err(format!(
            "refusing to recreate unexpected directory: {}",
            directory.display()
        )
        .into());
    }
    if directory.exists() {
        fs::remove_dir_all(directory)?;
    }
    fs::create_dir_all(directory)?;
    Ok(())
}

fn maven_program() -> &'static str {
    if cfg!(windows) {
        "mvn.cmd"
    } else {
        "mvn"
    }
}

fn run_checked(
    root: &Path,
    program: &str,
    args: &[&str],
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("starting {label} with {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}").into())
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file()
            && candidate.join("crates").join(RUST_CRATE).is_dir()
        {
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
    fn compare_reports_changed_generated_files() {
        let first = BTreeMap::from([("wire.rs".to_owned(), b"one".to_vec())]);
        let second = BTreeMap::from([("wire.rs".to_owned(), b"two".to_vec())]);
        let error = compare("Rust", &first, &second).unwrap_err().to_string();
        assert!(error.contains("changed=[\"wire.rs\"]"));
    }

    #[test]
    fn generation_paths_are_present() {
        let root = workspace_root().unwrap();
        assert!(root.join(JAVA_POM).is_file());
        assert!(root
            .join("crates")
            .join(RUST_CRATE)
            .join("build.rs")
            .is_file());
    }

    #[test]
    fn rust_output_uses_only_the_exact_package_build_script() {
        let unrelated =
            r#"{"reason":"build-script-executed","package_id":"other","out_dir":"dependency/out"}"#;
        let artifact = r#"{"reason":"compiler-artifact","package_id":"sdk"}"#;
        let expected = r#"{"reason":"build-script-executed","package_id":"sdk","out_dir":"sdk/build/hash/out"}"#;
        assert_eq!(
            rust_output_dir(&format!("{unrelated}\n{artifact}\n{expected}"), "sdk").unwrap(),
            PathBuf::from("sdk/build/hash/out"),
        );
        assert!(rust_output_dir(&format!("{unrelated}\n{artifact}"), "sdk").is_err());
        assert!(rust_output_dir(&format!("{expected}\n{expected}"), "sdk").is_err());
    }
}
