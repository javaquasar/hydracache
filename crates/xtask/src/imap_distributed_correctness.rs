use std::error::Error;
use std::path::PathBuf;
use std::process::Command;

const RELEASE: &str = "0.75";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let evidence = parse_args(&args)?;
    let root = crate::doc_check::find_repo_root()?;
    let status = Command::new("cargo")
        .args([
            "test",
            "-p",
            "hydracache-cluster-testkit",
            "--test",
            "canonical_key_075",
            "--test",
            "distributed_value_plane_075",
            "--test",
            "provisional_backend_075",
            "--test",
            "value_plane_admission_075",
            "--test",
            "value_plane_ack_075",
            "--test",
            "value_plane_bulk_075",
            "--test",
            "value_plane_explorer_075",
            "--test",
            "value_plane_listener_075",
            "--test",
            "value_plane_model_075",
            "--test",
            "value_plane_security_075",
            "--test",
            "value_plane_surface_075",
            "--test",
            "value_plane_transfer_075",
            "--locked",
        ])
        .current_dir(&root)
        .status()?;
    if !status.success() {
        return Err("local distributed correctness suite failed".into());
    }
    let contract_problems = crate::imap_contract::check_at_root(&root, RELEASE)?;
    if !contract_problems.is_empty() {
        return Err(format!("IMap contract problems: {contract_problems:?}").into());
    }
    let evidence_problems = crate::imap_foundation_evidence::check_at_root(&root, RELEASE, None)?;
    if !evidence_problems.is_empty() {
        return Err(format!("IMap evidence contract problems: {evidence_problems:?}").into());
    }
    if let Some(output) = evidence {
        crate::imap_foundation_generate::generate_at_root(&root, &output, 117)?;
        let problems =
            crate::imap_foundation_evidence::check_receipt_set_at_root(&root, RELEASE, &output)?;
        if !problems.is_empty() {
            return Err(format!("generated evidence set problems: {problems:?}").into());
        }
    }
    println!(
        "imap-distributed-correctness {RELEASE}: OK (local provisional proofs; production disabled)"
    );
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let mut release = None;
    let mut evidence = None;
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--release" => release = Some(value.clone()),
            "--evidence" => evidence = Some(PathBuf::from(value)),
            other => {
                return Err(
                    format!("unknown imap-distributed-correctness argument: {other}").into(),
                )
            }
        }
        index += 1;
    }
    if release.as_deref() != Some(RELEASE) {
        return Err("imap-distributed-correctness requires --release 0.75".into());
    }
    Ok(evidence)
}
