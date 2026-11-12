//! Clean second-directory reproduction of the two development Java packages.
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JavaPackageProof {
    pub component: String,
    pub sha256: String,
}

fn copy_sources(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    let mut pending = vec![(source.to_path_buf(), destination.to_path_buf(), 0usize)];
    let mut files = 0;
    let mut bytes = 0;
    let mut examined = 0;
    while let Some((source, destination, depth)) = pending.pop() {
        examined += 1;
        if examined > 1024 {
            return Err("package source total work bound".into());
        }
        let metadata = fs::symlink_metadata(&source)?;
        if depth > 16 || metadata.file_type().is_symlink() {
            return Err("package source depth/symlink bound".into());
        }
        if metadata.is_dir() {
            fs::create_dir_all(&destination)?;
            for entry in fs::read_dir(source)? {
                let entry = entry?;
                pending.push((entry.path(), destination.join(entry.file_name()), depth + 1));
                if pending.len() > 512 {
                    return Err("package source directory bound".into());
                }
            }
        } else if metadata.is_file() {
            if files >= 512 || metadata.len() > 2 * 1024 * 1024 {
                return Err("package source file/byte bound".into());
            }
            files += 1;
            bytes += metadata.len();
            if bytes > 32 * 1024 * 1024 {
                return Err("package source aggregate byte bound".into());
            }
            let mut contents = Vec::new();
            fs::File::open(source)?
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut contents)?;
            if contents.len() as u64 != metadata.len() {
                return Err("package source changed while copying".into());
            }
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)?;
            output.write_all(&contents)?;
        } else {
            return Err("unsupported package source file type".into());
        }
    }
    Ok(())
}

pub fn reproduce(root: &Path) -> Result<Vec<JavaPackageProof>, Box<dyn Error>> {
    let mut builds = crate::imap_build_process_075::BuildController::new(root)?;
    reproduce_with_controller(root, &mut builds)
}
pub fn reproduce_with_controller(
    root: &Path,
    builds: &mut crate::imap_build_process_075::BuildController,
) -> Result<Vec<JavaPackageProof>, Box<dyn Error>> {
    reproduce_packages(root, builds, crate::imap_sbom_capture_075::maven_command)
}
fn reproduce_packages(
    root: &Path,
    builds: &mut crate::imap_build_process_075::BuildController,
    mut maven: impl FnMut(&Path) -> Result<std::process::Command, Box<dyn Error>>,
) -> Result<Vec<JavaPackageProof>, Box<dyn Error>> {
    let scratch = root.join("target/imap-075");
    fs::create_dir_all(&scratch)?;
    let temp = tempfile::tempdir_in(scratch)?;
    let mut proofs = Vec::new();
    for (module, name) in [
        ("java-imap-benchmark", "java-imap-benchmark"),
        ("java-imap-hazelcast-075", "java-imap-hazelcast-075"),
    ] {
        let source = root.join("tests").join(module);
        let destination = temp.path().join(module);
        fs::create_dir(&destination)?;
        copy_sources(&source.join("pom.xml"), &destination.join("pom.xml"))?;
        copy_sources(&source.join("src"), &destination.join("src"))?;
        let mut invocation = maven(root)?;
        invocation
            .current_dir(root)
            .arg("-B")
            .arg("-f")
            .arg(destination.join("pom.xml"))
            .arg("package");
        builds.run(
            if module == "java-imap-benchmark" {
                crate::imap_build_process_075::BuildStage::ReproduceContract
            } else {
                crate::imap_build_process_075::BuildStage::ReproduceNative
            },
            invocation,
        )?;
        let artifact = format!("target/{name}-0.75.0-SNAPSHOT.jar");
        let original = crate::imap_daemon_probe_075::binary_digest(&source.join(&artifact))?;
        let rebuilt = crate::imap_daemon_probe_075::binary_digest(&destination.join(&artifact))?;
        if original != rebuilt {
            return Err(format!("clean Java package differs: {name}").into());
        }
        proofs.push(JavaPackageProof {
            component: module.into(),
            sha256: rebuilt,
        });
    }
    Ok(proofs)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_snapshot_is_bounded_and_never_overwrites_an_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("Sample.java"), b"source").unwrap();
        let target = root.path().join("target");
        copy_sources(&source, &target).unwrap();
        assert_eq!(fs::read(target.join("Sample.java")).unwrap(), b"source");
        assert!(copy_sources(&source, &target).is_err());
        fs::write(source.join("huge.java"), vec![0; 2 * 1024 * 1024 + 1]).unwrap();
        assert!(copy_sources(&source, &root.path().join("over-budget")).is_err());
        fs::remove_file(source.join("huge.java")).unwrap();
        let mut deep = source.clone();
        for _ in 0..17 {
            deep = deep.join("nested");
            fs::create_dir(&deep).unwrap();
        }
        assert!(copy_sources(&source, &root.path().join("deep-target")).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn source_snapshot_refuses_symlinked_inputs() {
        let root = tempfile::tempdir().unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(root.path(), &link).unwrap();
        assert!(copy_sources(&link, &root.path().join("target")).is_err());
    }
    #[test]
    fn source_snapshot_bounds_total_directory_work_with_a_small_pending_stack() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        for bucket in 0..12 {
            let directory = source.join(bucket.to_string());
            fs::create_dir(&directory).unwrap();
            for index in 0..100 {
                fs::create_dir(directory.join(index.to_string())).unwrap();
            }
        }
        assert!(copy_sources(&source, &root.path().join("target"))
            .unwrap_err()
            .to_string()
            .contains("total work"));
    }
}

#[cfg(test)]
#[path = "../tests/support/imap_package_command_075.rs"]
mod command_tests;
