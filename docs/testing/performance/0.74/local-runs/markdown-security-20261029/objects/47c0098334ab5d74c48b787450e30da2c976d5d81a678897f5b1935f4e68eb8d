//! One unprofiled fixture, not a finite-cohort coordinator or qualification.
use get_owner_scheduled_controls_074::security::{MaterialSeal, MtlsFixture};
use get_owner_scheduled_controls_074::timing::{self, Input};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, process::ExitCode};

enum Arguments<'a> {
    Validate(&'a str),
    GenerateMaterial(&'a str),
    ValidateMaterial {
        path: &'a str,
        pin: &'a str,
    },
    Run {
        config: &'a str,
        binary: &'a str,
        source: &'a str,
        material: Option<(&'a str, &'a str)>,
    },
}
fn digest_shape(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn arguments(args: &[String]) -> Result<Arguments<'_>, String> {
    match args {
        [mode, config] if mode == "--validate" => Ok(Arguments::Validate(config)),
        [mode, path] if mode == "--generate-material" => Ok(Arguments::GenerateMaterial(path)),
        [mode, path, pin] if mode == "--validate-material" && digest_shape(pin, 64) =>
            Ok(Arguments::ValidateMaterial { path, pin }),
        [mode, config, binary, source] if mode == "--run" && digest_shape(binary, 64) && digest_shape(source, 40) =>
            Ok(Arguments::Run { config, binary, source, material: None }),
        [mode, config, binary, source, path, pin] if mode == "--run-with-material" && digest_shape(binary, 64) && digest_shape(source, 40) && digest_shape(pin, 64) =>
            Ok(Arguments::Run { config, binary, source, material: Some((path, pin)) }),
        _ => Err("expected --validate CONFIG, --run CONFIG BINARY_SHA256 SOURCE_SHA, --generate-material PATH, --validate-material PATH MATERIAL_SHA256 or --run-with-material CONFIG BINARY_SHA256 SOURCE_SHA PATH MATERIAL_SHA256; no retry/defaults".to_owned()),
    }
}
fn execution_features_valid(allocation_diagnostics: bool) -> Result<(), String> {
    if allocation_diagnostics {
        Err("timing executable refuses allocation-diagnostics builds".to_owned())
    } else {
        Ok(())
    }
}
fn input(path: &Path) -> Result<Input, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    file.take(65537)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > 65536 {
        return Err("timing config exceeds 64 KiB".to_owned());
    }
    let config: Input = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    config.validate()?;
    Ok(config)
}
fn binary_hash(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = [0_u8; 65536];
    let mut hash = Sha256::new();
    loop {
        let len = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if len == 0 {
            break;
        }
        hash.update(&buffer[..len]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
#[derive(Serialize)]
struct Envelope<'a> {
    source_commit_from_coordinator: &'a str,
    source_git_identity_verified_by_binary: bool,
    binary_sha256: String,
    compiled_root_lock_sha256: String,
    compiled_observer_lock_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_material: Option<MaterialSeal>,
    report: Option<timing::Report>,
    error: Option<String>,
}
fn execute(args: &[String]) -> Result<bool, String> {
    let args = arguments(args)?;
    let (config, binary, source, material) = match args {
        Arguments::GenerateMaterial(path) => {
            let seal = MtlsFixture::new()?.save_new(Path::new(path))?;
            println!(
                "{}",
                serde_json::json!({"fixed_material": seal, "fixture_started": false, "admission_allowed": false})
            );
            return Ok(true);
        }
        Arguments::ValidateMaterial { path, pin } => {
            let (_, seal) = MtlsFixture::read_pinned(Path::new(path), pin)?;
            println!(
                "{}",
                serde_json::json!({"fixed_material": seal, "fixture_started": false, "admission_allowed": false})
            );
            return Ok(true);
        }
        Arguments::Validate(path) => {
            let config = input(Path::new(path))?;
            println!(
                "{}",
                serde_json::json!({"valid": true, "workload_sha256": config.workload_sha256(),
                "fixture_started": false, "admission_allowed": false})
            );
            return Ok(true);
        }
        Arguments::Run {
            config,
            binary,
            source,
            material,
        } => (config, binary, source, material),
    };
    execution_features_valid(cfg!(feature = "allocation-diagnostics"))?;
    let config = input(Path::new(config))?;
    let hash = binary_hash(&std::env::current_exe().map_err(|e| e.to_string())?)?;
    if hash != binary {
        return Err("timing binary seal mismatch before fixture".to_owned());
    }
    if material.is_some() && !config.surface.uses_mtls() {
        return Err("fixed material requires an mTLS surface before fixture".to_owned());
    }
    let fixed = material
        .map(|(path, pin)| MtlsFixture::read_pinned(Path::new(path), pin))
        .transpose()?;
    let mut envelope = Envelope {
        source_commit_from_coordinator: source,
        source_git_identity_verified_by_binary: false,
        binary_sha256: hash,
        compiled_root_lock_sha256: format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../../../Cargo.lock"))
        ),
        compiled_observer_lock_sha256: format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../Cargo.lock"))
        ),
        fixed_material: fixed.as_ref().map(|(_, seal)| seal.clone()),
        report: None,
        error: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    // A process-level failure is not a completed/explicitly-shut-down receipt.
    // An external coordinator must own the child and enforce its hard deadline.
    match runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(60), async {
            match fixed {
                Some((material, _)) => timing::run_with_material(config, &material).await,
                None => timing::run(config).await,
            }
        })
        .await
    }) {
        Ok(report) => envelope.report = Some(report),
        Err(_) => envelope.error = Some("process fixture deadline; no shutdown proof".to_owned()),
    }
    let ok = envelope.error.is_none()
        && envelope
            .report
            .as_ref()
            .is_some_and(|r| r.error.is_none() && r.shutdown_verified);
    println!(
        "{}",
        serde_json::to_string(&envelope).map_err(|e| e.to_string())?
    );
    Ok(ok)
}
fn main() -> ExitCode {
    match execute(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_need_exact_binary_and_full_source_identity_without_retry() {
        let config = "fixture.json".to_owned();
        let binary = "a".repeat(64);
        let source = "b".repeat(40);
        assert!(arguments(&[
            "--run".to_owned(),
            config.clone(),
            binary.clone(),
            source.clone()
        ])
        .is_ok());
        for args in [
            vec![],
            vec!["--run".to_owned(), config.clone()],
            vec![
                "--run".to_owned(),
                config.clone(),
                "a".repeat(63),
                source.clone(),
            ],
            vec![
                "--run".to_owned(),
                config.clone(),
                binary.clone(),
                "B".repeat(40),
            ],
            vec![
                "--run".to_owned(),
                config.clone(),
                binary,
                source,
                "retry".to_owned(),
            ],
        ] {
            assert!(arguments(&args).is_err());
        }
        assert!(arguments(&["--validate".to_owned(), config]).is_ok());
        assert!(arguments(&["--generate-material".to_owned(), "private.json".to_owned()]).is_ok());
        assert!(arguments(&[
            "--validate-material".to_owned(),
            "private.json".to_owned(),
            "a".repeat(64)
        ])
        .is_ok());
        assert!(arguments(&[
            "--run-with-material".to_owned(),
            "config.json".to_owned(),
            "a".repeat(64),
            "b".repeat(40),
            "private.json".to_owned(),
            "c".repeat(64)
        ])
        .is_ok());
        assert!(arguments(&[
            "--validate-material".to_owned(),
            "private.json".to_owned(),
            "A".repeat(64)
        ])
        .is_err());
        assert!(execution_features_valid(true).is_err());
        execution_features_valid(false).unwrap();
    }
    #[test]
    fn binary_seal_hashes_raw_bytes_and_missing_file_fails() {
        let fixture = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(fixture.path(), b"timing\r\nraw").unwrap();
        assert_eq!(
            binary_hash(fixture.path()).unwrap(),
            format!("{:x}", Sha256::digest(b"timing\r\nraw"))
        );
        assert!(binary_hash(Path::new("not-a-timing-binary-074")).is_err());
        assert!(input(fixture.path()).is_err());
        std::fs::write(fixture.path(), vec![b' '; 65537]).unwrap();
        assert!(input(fixture.path()).unwrap_err().contains("64 KiB"));
    }
}
