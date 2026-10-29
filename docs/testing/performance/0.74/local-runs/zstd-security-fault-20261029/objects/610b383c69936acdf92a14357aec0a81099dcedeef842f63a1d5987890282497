use hydracache_long_run_supervisor_074::diagnostic_artifacts::ArtifactDigest;
use hydracache_long_run_supervisor_074::diagnostic_signer_bootstrap_context::{
    self as context, AssertedBootstrapContext,
};
use hydracache_long_run_supervisor_074::diagnostic_signer_build::SignerBuildStatement;

// All pins/requirements here are invented test assertions, not production choices.
pub fn synthetic_context(s: &SignerBuildStatement) -> AssertedBootstrapContext {
    AssertedBootstrapContext {
        schema_version: context::SCHEMA.into(),
        repository_id: s.repository_id,
        source_commit: s.source_commit.clone(),
        source_tree: s.source_tree.clone(),
        binary: s.binary.clone(),
        runtime_manifest: s.runtime_manifest.clone(),
        launcher: ArtifactDigest::of(b"synthetic separate launcher"),
        workflow_ref: context::WORKFLOW_REF.into(),
        workflow_sha: "c".repeat(40),
        environment: context::ENVIRONMENT.into(),
        subject: context::SUBJECT.into(),
        event_name: "workflow_dispatch".into(),
        runner_environment: "github-hosted".into(),
        build_has_signing_authority: false,
        build_has_oidc_authority: false,
        signer_compilation_allowed: false,
        signer_dependency_install_allowed: false,
        signer_checkout_allowed: false,
        inherits_unreviewed_environment: false,
    }
}
