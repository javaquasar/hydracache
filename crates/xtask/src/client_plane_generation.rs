use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    let rust_a = generate_rust(root, &generation_root.join("rust-a"))?;
    let rust_b = generate_rust(root, &generation_root.join("rust-b"))?;
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
) -> Result<BTreeMap<String, Vec<u8>>, Box<dyn Error>> {
    let target = target_dir.to_string_lossy().into_owned();
    run_checked(
        root,
        "cargo",
        &[
            "check",
            "--locked",
            "-p",
            RUST_CRATE,
            "--target-dir",
            &target,
        ],
        "clean Rust HC/2 generation",
    )?;
    let build_root = target_dir.join("debug").join("build");
    let mut generated = BTreeMap::new();
    for entry in fs::read_dir(&build_root)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with(RUST_CRATE) {
            continue;
        }
        let out = entry.path().join("out");
        if out.is_dir() {
            collect_files(&out, &out, &mut generated)?;
        }
    }
    require_non_empty("Rust", generated)
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
}
