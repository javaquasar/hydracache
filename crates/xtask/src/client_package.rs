use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tar::Archive;

const API_MANIFEST: &str = "docs/compatibility/hc2-sdk-api-v1.json";
const SCRATCH: &str = "target/hc2-package-check";
const RUST_CRATE: &str = "hydracache-client-hc2";
const JAVA_REACTOR: &str = "sdks/java/pom.xml";
const JAVA_CONSUMER: &str = "tests/java-hazelcast-facade-consumer/pom.xml";
const PYTHON_PACKAGE: &str = "sdks/python/hydracache-client-hc2";

#[derive(Debug, Deserialize)]
struct ApiManifest {
    schema_version: u32,
    stability: String,
    protocol_generation: u32,
    packages: Vec<ApiPackage>,
}

#[derive(Debug, Deserialize)]
struct ApiPackage {
    ecosystem: String,
    coordinate: String,
    version: String,
    public_symbols: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-package-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let manifest: ApiManifest = serde_json::from_slice(&fs::read(root.join(API_MANIFEST))?)?;
    validate_manifest(&manifest)?;
    let scratch = root.join(SCRATCH);
    reset_scratch(root, &scratch)?;
    let rust_version = workspace_version(root)?;
    check_rust(
        root,
        &scratch,
        package(&manifest, "rust", RUST_CRATE)?,
        &rust_version,
    )?;
    check_java(root, &manifest)?;
    check_python(root, &scratch, package(&manifest, "python", RUST_CRATE)?)?;
    println!(
        "client-package-check: OK (frozen API manifest + clean Rust crate, Java JAR, and Python wheel consumers)"
    );
    Ok(())
}

fn validate_manifest(manifest: &ApiManifest) -> Result<(), Box<dyn Error>> {
    if manifest.schema_version != 1
        || manifest.stability != "preview-frozen"
        || manifest.protocol_generation != 6
        || manifest.packages.len() != 4
    {
        return Err("HC/2 SDK API manifest header is not the frozen v1 contract".into());
    }
    for package in &manifest.packages {
        if package.coordinate.trim().is_empty()
            || package.version.trim().is_empty()
            || package.public_symbols.is_empty()
        {
            return Err(format!("incomplete API package row: {}", package.coordinate).into());
        }
        let mut sorted = package.public_symbols.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != package.public_symbols.len() {
            return Err(format!("duplicate API symbol in {}", package.coordinate).into());
        }
    }
    Ok(())
}

fn package<'a>(
    manifest: &'a ApiManifest,
    ecosystem: &str,
    coordinate: &str,
) -> Result<&'a ApiPackage, Box<dyn Error>> {
    manifest
        .packages
        .iter()
        .find(|package| package.ecosystem == ecosystem && package.coordinate == coordinate)
        .ok_or_else(|| format!("missing {ecosystem} API row for {coordinate}").into())
}

fn check_rust(
    root: &Path,
    scratch: &Path,
    api: &ApiPackage,
    rust_version: &str,
) -> Result<(), Box<dyn Error>> {
    if api.version != rust_version {
        return Err("Rust API manifest version drifted from the workspace package".into());
    }
    run_checked(
        root,
        "cargo",
        &[
            "package",
            "--locked",
            "-p",
            RUST_CRATE,
            "--allow-dirty",
            "--no-verify",
        ],
        "Rust HC/2 package",
    )?;
    let archive = root
        .join("target/package")
        .join(format!("{RUST_CRATE}-{rust_version}.crate"));
    if !archive.is_file() {
        return Err(format!("Rust package archive is missing: {}", archive.display()).into());
    }
    let unpacked = scratch.join("rust-package");
    fs::create_dir_all(&unpacked)?;
    Archive::new(GzDecoder::new(fs::File::open(&archive)?)).unpack(&unpacked)?;
    let package_root = unpacked.join(format!("{RUST_CRATE}-{rust_version}"));
    let consumer = scratch.join("rust-consumer");
    fs::create_dir_all(consumer.join("src"))?;
    let dependency_path = package_root.to_string_lossy().replace('\\', "/");
    fs::write(
        consumer.join("Cargo.toml"),
        format!(
            "[package]\nname = \"hc2-package-consumer\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[dependencies]\nhydracache-client-hc2 = {{ path = \"{dependency_path}\" }}\n"
        ),
    )?;
    let imports = api.public_symbols.join(", ");
    let assertions = api
        .public_symbols
        .iter()
        .map(|symbol| format!("    let _ = std::any::type_name::<{symbol}>();\n"))
        .collect::<String>();
    fs::write(
        consumer.join("src/main.rs"),
        format!("use hydracache_client_hc2::{{{imports}}};\nfn main() {{\n{assertions}}}\n"),
    )?;
    let manifest = consumer.join("Cargo.toml").to_string_lossy().into_owned();
    run_checked(
        root,
        "cargo",
        &["check", "--offline", "--manifest-path", &manifest],
        "clean Rust HC/2 package consumer",
    )
}

fn workspace_version(root: &Path) -> Result<String, Box<dyn Error>> {
    let manifest: toml::Value = toml::from_str(&fs::read_to_string(root.join("Cargo.toml"))?)?;
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "workspace.package.version is missing from Cargo.toml".into())
}

fn check_java(root: &Path, manifest: &ApiManifest) -> Result<(), Box<dyn Error>> {
    for api in manifest
        .packages
        .iter()
        .filter(|package| package.ecosystem == "java")
    {
        let package_dir = if api.coordinate.ends_with("hydracache-client-hc2") {
            root.join("sdks/java/hydracache-client-hc2/src/main/java/io/hydracache/client/hc2")
        } else {
            root.join("sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast")
        };
        for symbol in &api.public_symbols {
            if !package_dir.join(format!("{symbol}.java")).is_file() {
                return Err(format!(
                    "Java API symbol {symbol} is missing from {}",
                    api.coordinate
                )
                .into());
            }
        }
    }
    run_checked(
        root,
        maven_program(),
        &["-B", "-ntp", "-f", JAVA_REACTOR, "-DskipTests", "install"],
        "Java HC/2 reactor package install",
    )?;
    run_checked(
        root,
        maven_program(),
        &["-B", "-ntp", "-f", JAVA_CONSUMER, "verify"],
        "clean Java facade package consumer",
    )
}

fn check_python(root: &Path, scratch: &Path, api: &ApiPackage) -> Result<(), Box<dyn Error>> {
    let python = python_program();
    let package = root.join(PYTHON_PACKAGE).to_string_lossy().into_owned();
    let first = scratch.join("python-dist-a");
    let second = scratch.join("python-dist-b");
    fs::create_dir_all(&first)?;
    fs::create_dir_all(&second)?;
    for output in [&first, &second] {
        let output = output.to_string_lossy().into_owned();
        run_checked(
            root,
            python,
            &[
                "-m",
                "pip",
                "wheel",
                "--no-index",
                "--no-cache-dir",
                "--no-deps",
                "--no-build-isolation",
                "--wheel-dir",
                &output,
                &package,
            ],
            "deterministic Python HC/2 wheel build",
        )?;
    }
    let wheel_name = format!("hydracache_client_hc2-{}-py3-none-any.whl", api.version);
    let wheel_a = first.join(&wheel_name);
    let wheel_b = second.join(&wheel_name);
    if sha256(&wheel_a)? != sha256(&wheel_b)? {
        return Err("Python HC/2 wheel is not byte-for-byte deterministic".into());
    }
    let venv = scratch.join("python-consumer");
    let venv_path = venv.to_string_lossy().into_owned();
    run_checked(
        root,
        python,
        &["-m", "venv", "--clear", &venv_path],
        "Python consumer venv",
    )?;
    let venv_python = if cfg!(windows) {
        venv.join("Scripts/python.exe")
    } else {
        venv.join("bin/python")
    };
    let venv_python = venv_python.to_string_lossy().into_owned();
    let requirements = root
        .join(PYTHON_PACKAGE)
        .join("requirements.lock")
        .to_string_lossy()
        .into_owned();
    let wheelhouse = root
        .join(PYTHON_PACKAGE)
        .join("wheelhouse")
        .to_string_lossy()
        .into_owned();
    run_checked(
        root,
        &venv_python,
        &[
            "-m",
            "pip",
            "install",
            "--no-index",
            "--require-hashes",
            "--find-links",
            &wheelhouse,
            "-r",
            &requirements,
        ],
        "offline Python HC/2 runtime dependencies",
    )?;
    let wheel_a = wheel_a.to_string_lossy().into_owned();
    run_checked(
        root,
        &venv_python,
        &["-m", "pip", "install", "--no-index", "--no-deps", &wheel_a],
        "clean Python HC/2 wheel install",
    )?;
    let expected = serde_json::to_string(&api.public_symbols)?;
    let expected_version = serde_json::to_string(&api.version)?;
    let script = format!(
        "import hydracache_hc2 as h; assert h.__version__ == {expected_version}; assert sorted(h.__all__) == sorted({expected}); print('python package consumer: OK')"
    );
    run_checked(
        root,
        &venv_python,
        &["-I", "-c", &script],
        "clean Python HC/2 wheel consumer",
    )
}

fn sha256(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut hasher = Sha256::new();
    hasher.update(fs::read(path)?);
    Ok(hasher.finalize().to_vec())
}

fn reset_scratch(root: &Path, scratch: &Path) -> Result<(), Box<dyn Error>> {
    let target = root.join("target").canonicalize()?;
    let resolved = scratch
        .parent()
        .ok_or("package scratch has no parent")?
        .canonicalize()?;
    if resolved != target {
        return Err("refusing to reset package scratch outside workspace target".into());
    }
    if scratch.exists() {
        fs::remove_dir_all(scratch)?;
    }
    fs::create_dir_all(scratch)?;
    Ok(())
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
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}").into())
    }
}

fn maven_program() -> &'static str {
    if cfg!(windows) {
        "mvn.cmd"
    } else {
        "mvn"
    }
}

fn python_program() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join(API_MANIFEST).is_file() {
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
    fn frozen_manifest_names_all_four_preview_packages() {
        let root = workspace_root().unwrap();
        let manifest: ApiManifest =
            serde_json::from_slice(&fs::read(root.join(API_MANIFEST)).unwrap()).unwrap();
        validate_manifest(&manifest).unwrap();
        assert_eq!(manifest.packages.len(), 4);
    }

    #[test]
    fn rust_api_manifest_tracks_the_workspace_release_version() {
        let root = workspace_root().unwrap();
        let manifest: ApiManifest =
            serde_json::from_slice(&fs::read(root.join(API_MANIFEST)).unwrap()).unwrap();
        let rust = package(&manifest, "rust", RUST_CRATE).unwrap();
        assert_eq!(rust.version, workspace_version(&root).unwrap());
    }
}
