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
            "--lib",
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
            "value_plane_async_075",
            "--test",
            "value_plane_bulk_075",
            "--test",
            "value_plane_bulk_control_075",
            "--test",
            "value_plane_collection_075",
            "--test",
            "value_plane_concurrent_075",
            "--test",
            "value_plane_capability_075",
            "--test",
            "value_plane_differential_075",
            "--test",
            "value_plane_extended_075",
            "--test",
            "value_plane_cluster_evolution_075",
            "--test",
            "value_plane_cluster_helpers_075",
            "--test",
            "value_plane_dedup_075",
            "--test",
            "value_plane_durable_075",
            "--test",
            "value_plane_explorer_075",
            "--test",
            "value_plane_listener_075",
            "--test",
            "value_plane_loopback_075",
            "--test",
            "value_plane_model_075",
            "--test",
            "value_plane_security_075",
            "--test",
            "value_plane_service_075",
            "--test",
            "value_plane_service_cluster_075",
            "--test",
            "value_plane_cluster_admission_075",
            "--test",
            "value_plane_cluster_chaos_075",
            "--test",
            "value_plane_cluster_runtime_075",
            "--test",
            "value_plane_cluster_subscriptions_075",
            "--test",
            "value_plane_online_transfer_075",
            "--test",
            "value_plane_service_registry_075",
            "--test",
            "value_plane_service_lifecycle_075",
            "--test",
            "value_plane_service_async_075",
            "--test",
            "value_plane_stream_075",
            "--test",
            "value_plane_resources_075",
            "--test",
            "value_plane_subscriptions_075",
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
    let java_status = Command::new(maven_program())
        .args(["-B", "-f", "tests/java-imap-benchmark/pom.xml", "test"])
        .current_dir(&root)
        .status()?;
    if !java_status.success() {
        return Err("Java IMap semantic suite failed".into());
    }
    let differential = crate::imap_api_differential::check_actual_at_root(&root, 117, 128)?;
    println!("imap-api-differential: {differential:?}");
    let contract_problems = crate::imap_contract::check_at_root(&root, RELEASE)?;
    if !contract_problems.is_empty() {
        return Err(format!("IMap contract problems: {contract_problems:?}").into());
    }
    let evidence_problems = crate::imap_foundation_evidence::check_at_root(&root, RELEASE, None)?;
    if !evidence_problems.is_empty() {
        return Err(format!("IMap evidence contract problems: {evidence_problems:?}").into());
    }
    let capability_problems = crate::imap_capability_audit::check_at_root(&root, RELEASE)?;
    if !capability_problems.is_empty() {
        return Err(format!("IMap capability drift: {capability_problems:?}").into());
    }
    crate::imap_migration_dry_run::check_at_root(&root, RELEASE)?;
    let semantic_problems = crate::imap_semantic_audit075::check_at_root(&root, RELEASE)?;
    if !semantic_problems.is_empty() {
        return Err(format!("IMap semantic compatibility drift: {semantic_problems:?}").into());
    }
    println!(
        "actual cluster campaign: {}",
        crate::imap_cluster_campaign_075::check_at_root(&root, 117, 3)?
    );
    let fuzz_problems = crate::imap_cluster_fuzz_policy075::check_at_root(&root)?;
    if !fuzz_problems.is_empty() {
        return Err(format!("cluster fuzz policy drift: {fuzz_problems:?}").into());
    }
    crate::imap_ci_075::check_workflow(&root)?;
    let comparison_blockers = crate::imap_qualification_075::check_comparison_at_root(&root)?;
    let admission = crate::imap_qualification_075::check_release_at_root(&root, None)?;
    println!(
        "IMap release preparation: contracts valid; comparison blockers={}, ship admitted={}",
        comparison_blockers.len(),
        admission.admitted
    );
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

fn maven_program() -> &'static str {
    if cfg!(windows) {
        "mvn.cmd"
    } else {
        "mvn"
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_maven_program_is_executable_name() {
        assert!(matches!(maven_program(), "mvn" | "mvn.cmd"));
    }

    #[test]
    fn arguments_require_the_exact_provisional_release() {
        assert!(parse_args(&["--release".into(), "0.75".into()]).is_ok());
        assert!(parse_args(&["--release".into(), "0.74".into()]).is_err());
        assert!(parse_args(&["--unknown".into(), "value".into()]).is_err());
        assert!(parse_args(&["--release".into()]).is_err());
    }
}
