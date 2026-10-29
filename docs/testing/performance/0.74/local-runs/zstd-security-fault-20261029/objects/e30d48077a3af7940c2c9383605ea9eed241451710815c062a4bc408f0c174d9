//! Test-only signed assertions, not a production clean-build attestation.
use super::*;
use crate::diagnostic_builder::bootstrap_build as build;

impl Fixture {
    pub(crate) fn bootstrap_for_service(&self) -> (File, Vec<u8>, Vec<u8>) {
        let image =
            std::fs::read("/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074").unwrap();
        assert!(image.len() <= 32 * 1024 * 1024);
        let held: held::Policy = serde_json::from_slice(&self.held_policy).unwrap();
        let controller = crate::diagnostic_builder::decode_key(&held.controller_key_hex).unwrap();
        let builder = SigningKey::from_bytes(&[83; 32]);
        let builder_key = builder.verifying_key();
        let policy = build::Policy {
            schema_version: build::POLICY_SCHEMA.into(),
            purpose: build::PURPOSE.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            builder_id: build::BUILDER_ID.into(),
            builder_key_hex: crate::hex(builder_key.as_bytes()),
            controller_key_hex: held.controller_key_hex.clone(),
            authority_epoch: held.authority_epoch,
            source: held.source,
            bootstrap_sha256: crate::sha256_hex(&image),
            held_policy_sha256: crate::sha256_hex(&self.held_policy),
            held_receipt_sha256: crate::sha256_hex(&self.receipt),
            original_host_sha256: self.artifacts.watchdog_sha256.clone(),
            held_launcher_sha256: self.artifacts.launcher_sha256.clone(),
        };
        let expected = build::ExpectedTrust {
            builder: &builder_key,
            controller: &controller,
            epoch: policy.authority_epoch,
            held_policy_sha256: &policy.held_policy_sha256,
            bootstrap_sha256: &policy.bootstrap_sha256,
        };
        let raw = held::document_bytes(&build::sign_policy(&policy, &builder, &expected).unwrap())
            .unwrap();
        let checked = build::load_policy(&raw, &crate::sha256_hex(&raw), &expected).unwrap();
        let log = [
            serde_json::json!({"reason":"compiler-artifact","package_id":"path+file:///fixture/tools/long-run-supervisor-074#0.0.0","target":{"name":build::BINARY,"kind":["bin"],"crate_types":["bin"],"src_path":"/fixture/tools/long-run-supervisor-074/src/bin/diagnostic_held_bootstrap.rs"},"profile":{"opt_level":"3","debug_assertions":false,"test":false},"features":[],"executable":"/fixture/target/x86_64-unknown-linux-gnu/release/diagnostic_held_bootstrap"}),
            serde_json::json!({"reason":"build-finished","success":true}),
        ].iter().flat_map(line).collect::<Vec<_>>();
        let receipt = held::document_bytes(
            &build::sign_build(
                &checked,
                &builder,
                &held::Observation {
                    source_commit_before: policy.source.commit.clone(),
                    source_commit_after: policy.source.commit.clone(),
                    source_tree_before: policy.source.tree.clone(),
                    source_tree_after: policy.source.tree.clone(),
                    source_clean_before: true,
                    source_clean_after: true,
                    isolated_build: true,
                    rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
                    cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
                    build_command: build::BUILD_COMMAND.map(String::from).to_vec(),
                },
                &build::Contents {
                    bootstrap: &image,
                    root_lock: b"synthetic-lock\n",
                    build_log: &log,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let enrollment = line(&serde_json::json!({
            "schema_version":"diagnostic-bootstrap-enrollment-074-v1",
            "repository_id":crate::diagnostic_builder::REPOSITORY_ID,
            "bootstrap_builder_key_hex":policy.builder_key_hex,
            "bootstrap_policy_sha256":crate::sha256_hex(&raw),
            "bootstrap_sha256":policy.bootstrap_sha256,
            "held_policy_sha256":policy.held_policy_sha256,
            "policy_epoch":policy.authority_epoch,
        }));
        // Same protected root as the consumed HeldTrustInput; never production /etc.
        let directory = self.root.path().join("etc/hydracache-perf");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(directory.join("held-bootstrap-enrollment-074.json"))
            .unwrap();
        file.write_all(&enrollment).unwrap();
        file.sync_all().unwrap();
        File::open(directory).unwrap().sync_all().unwrap();
        let image = sealed(&image);
        image
            .set_permissions(std::fs::Permissions::from_mode(0o500))
            .unwrap();
        (image, raw, receipt)
    }
}
