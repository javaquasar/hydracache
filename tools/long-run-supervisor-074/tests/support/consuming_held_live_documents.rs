//! Synthetic trust/build documents and actual sealed role FDs, test builds only.
use super::super::*;
use crate::diagnostic_two_phase_launch::{held_placement as wire, LaunchArtifacts};
use ed25519_dalek::{Signer, SigningKey};
use std::io::Write;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;

pub(crate) struct Fixture {
    root: tempfile::TempDir,
    pub(crate) input: Option<HeldTrustInput>,
    pub(crate) held_policy: Vec<u8>,
    pub(crate) worker_policy: Vec<u8>,
    pub(crate) receipt: Vec<u8>,
    pub(crate) roles: Option<held::role_files::PinnedRoles>,
    pub(crate) alias: File,
    pub(crate) artifacts: LaunchArtifacts,
    checked: held::CheckedPolicy,
    original_config: Vec<u8>,
}
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut raw = canonical_json(value).unwrap();
    raw.push(b'\n');
    raw
}
fn elf(tag: u8) -> Vec<u8> {
    let mut raw = vec![0; 128];
    raw[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    raw[16] = 2;
    raw[18] = 62;
    raw[20] = 1;
    raw[32] = 64;
    raw[52] = 64;
    raw[54] = 56;
    raw[56] = 1;
    raw[127] = tag;
    raw
}
fn sealed(raw: &[u8]) -> File {
    let fd = unsafe {
        libc::memfd_create(
            c"hc074-held-live".as_ptr(),
            libc::MFD_ALLOW_SEALING | libc::MFD_CLOEXEC,
        )
    };
    assert!(fd >= 0);
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.write_all(raw).unwrap();
    assert_eq!(
        unsafe {
            libc::fcntl(
                fd,
                libc::F_ADD_SEALS,
                libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE,
            )
        },
        0
    );
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(format!("/proc/self/fd/{fd}"))
        .unwrap()
}
impl Fixture {
    pub(crate) fn new(controller: VerifyingKey, worker_bytes: &[u8], worker_policy: &[u8]) -> Self {
        let worker: crate::diagnostic_worker_policy::SignedWorkerPolicy =
            serde_json::from_slice(worker_bytes).unwrap();
        let worker_builder: crate::diagnostic_builder::BuilderPolicy =
            serde_json::from_slice(worker_policy).unwrap();
        let builder = SigningKey::from_bytes(&[82; 32]);
        let lock = b"synthetic-lock\n";
        let held_policy = held::document_bytes(&held::Policy {
            schema_version: held::POLICY_SCHEMA.into(),
            purpose: held::PURPOSE.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            builder_id: held::BUILDER_ID.into(),
            builder_key_hex: crate::hex(builder.verifying_key().as_bytes()),
            controller_key_hex: crate::hex(controller.as_bytes()),
            authority_epoch: worker.policy.policy_epoch,
            source: held::Source {
                commit: "a".repeat(40),
                tree: "b".repeat(40),
                root_lock: crate::diagnostic_artifacts::ArtifactDigest::of(lock),
            },
        })
        .unwrap();
        let selected = Selection {
            controller,
            worker_builder: crate::diagnostic_builder::decode_key(&worker_builder.builder_key_hex)
                .unwrap(),
            worker_issuer: SigningKey::from_bytes(&[75; 32]).verifying_key(),
            worker_builder_policy_sha256: crate::sha256_hex(worker_policy),
        };
        let document = Document {
            schema_version: TRUST_SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            controller_key_hex: crate::hex(controller.as_bytes()),
            held_builder_key_hex: crate::hex(builder.verifying_key().as_bytes()),
            held_policy_sha256: crate::sha256_hex(&held_policy),
            worker_builder_policy_sha256: selected.worker_builder_policy_sha256.clone(),
            worker_policy_sha256: crate::sha256_hex(&line(&worker.policy)),
            policy_epoch: worker.policy.policy_epoch,
        };
        let root = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
        std::fs::create_dir(root.path().join("etc")).unwrap();
        std::fs::create_dir(root.path().join("etc/hydracache-perf")).unwrap();
        let original_config = line(&document);
        let path = root.path().join("etc/hydracache-perf").join(TRUST_FILE);
        std::fs::write(&path, &original_config).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let input = HeldTrustInput {
            source: HeldSource::open(root.path(), 0, selected).unwrap(),
            refused: false,
            _original_thread: PhantomData,
        };
        let checked = held::load_policy(
            &held_policy,
            &document.held_policy_sha256,
            &held::ExpectedTrust {
                controller_key: &controller,
                authority_epoch: document.policy_epoch,
                worker_policy,
                worker_policy_sha256: &document.worker_builder_policy_sha256,
            },
        )
        .unwrap();
        let host = elf(1);
        let launcher = elf(2);
        let mut log = Vec::new();
        for (name, source) in [
            (held::HOST, "main.rs"),
            (held::LAUNCHER, "bin/diagnostic_held_launcher.rs"),
        ] {
            log.extend(line(&serde_json::json!({"reason":"compiler-artifact","package_id":"path+file:///isolated/tools/long-run-supervisor-074#0.0.0","target":{"name":name,"kind":["bin"],"crate_types":["bin"],"src_path":format!("/isolated/tools/long-run-supervisor-074/src/{source}")},"profile":{"opt_level":"3","debug_assertions":false,"test":false},"features":[],"executable":format!("/target/x86_64-unknown-linux-gnu/release/{name}"),"fresh":false})));
        }
        log.extend(line(
            &serde_json::json!({"reason":"build-finished","success":true}),
        ));
        let receipt = line(
            &held::sign_build(
                &checked,
                &builder,
                &held::Observation {
                    source_commit_before: "a".repeat(40),
                    source_commit_after: "a".repeat(40),
                    source_tree_before: "b".repeat(40),
                    source_tree_after: "b".repeat(40),
                    source_clean_before: true,
                    source_clean_after: true,
                    isolated_build: true,
                    rustc_version: "rustc 1.94.0 (4a4ef493e 2026-03-02)".into(),
                    cargo_version: "cargo 1.94.0 (85eff7c80 2026-01-15)".into(),
                    build_command: held::BUILD_COMMAND.iter().map(|s| (*s).into()).collect(),
                },
                &held::Contents {
                    original_host: &host,
                    held_launcher: &launcher,
                    root_lock: lock,
                    build_log: &log,
                },
            )
            .unwrap(),
        );
        let host_fd = sealed(&host);
        let alias = host_fd.try_clone().unwrap();
        let launcher_fd = sealed(&launcher);
        let artifacts = LaunchArtifacts {
            launcher_sha256: crate::sha256_hex(&launcher),
            watchdog_sha256: crate::sha256_hex(&host),
            collector_sha256: crate::sha256_hex(&host),
        };
        let roles = held::verify_receipt(&receipt, &checked)
            .unwrap()
            .pin_sealed_roles(host_fd, launcher_fd, lock, &log, &artifacts)
            .unwrap();
        Self {
            root,
            input: Some(input),
            held_policy,
            worker_policy: worker_policy.into(),
            receipt,
            roles: Some(roles),
            alias,
            artifacts,
            checked,
            original_config,
        }
    }
    pub(crate) fn bind(
        &self,
        placement: wire::SignedPlacement,
        controller: &SigningKey,
    ) -> Vec<u8> {
        let body = role_binding::Body {
            schema_version: role_binding::BODY_SCHEMA.into(),
            action: "bind-held-build-to-placement".into(),
            placement,
            held_policy_sha256: crate::sha256_hex(&self.held_policy),
            held_receipt_sha256: crate::sha256_hex(&self.receipt),
            authority_epoch: self.checked_epoch(),
        };
        let signature_hex = crate::hex(
            &controller
                .sign(&role_binding::signing_message(&body, &self.checked, &self.receipt).unwrap())
                .to_bytes(),
        );
        line(&role_binding::SignedBinding {
            schema_version: role_binding::ENVELOPE_SCHEMA.into(),
            binding: body,
            signature_hex,
        })
    }
    fn checked_epoch(&self) -> u64 {
        let doc: held::Policy = serde_json::from_slice(&self.held_policy).unwrap();
        doc.authority_epoch
    }
    pub(crate) fn drift(&self, repair: bool) {
        std::fs::write(
            self.root
                .path()
                .join("etc/hydracache-perf")
                .join(TRUST_FILE),
            if repair {
                self.original_config.as_slice()
            } else {
                b"drift\n"
            },
        )
        .unwrap();
    }
}
