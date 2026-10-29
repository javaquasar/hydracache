//! Owned nonroot modeled roots and synthetic issuer only; not installed enrollment.
#[path = "diagnostic_privileged_worker_fixture.rs"]
pub(crate) mod privileged_fixture;
use super::super::authority_registry::{fixture, RefusalReason};
use super::super::{
    NamespaceIdentity, SignedWorkerPolicy, WorkerPolicy, SIGNED_WORKER_POLICY_SCHEMA,
    WORKER_POLICY_DOMAIN, WORKER_POLICY_SCHEMA,
};
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn uid() -> u32 {
    unsafe { libc::geteuid() }
}
fn gid() -> u32 {
    unsafe { libc::getegid() }
}
fn line(value: &impl Serialize) -> Vec<u8> {
    let mut raw = canonical_json(value).unwrap();
    raw.push(b'\n');
    raw
}
struct Model {
    root: TempDir,
    authority_root: TempDir,
    key: SigningKey,
    policy: WorkerPolicy,
    host: AssertedWorkerHost,
    envelope: Vec<u8>,
    trust: WorkerPolicyTrust,
}
impl Model {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.path().join("etc")).unwrap();
        fs::create_dir(root.path().join("etc/hydracache-perf")).unwrap();
        let key = SigningKey::from_bytes(&[75; 32]); // Test issuer, never the actual local private key.
        let policy = WorkerPolicy {
            schema_version: WORKER_POLICY_SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            purpose: "diagnostic-worker-only".into(),
            account_source: "local-files".into(),
            account: "hydracache-perf".into(),
            group: "hydracache-perf".into(),
            policy_epoch: 7,
            uid: 986,
            gid: 986,
            supplementary_gids: vec![987],
            machine_id: "a".repeat(32),
            boot_id: "11111111-2222-3333-4444-555555555555".into(),
            user_namespace: NamespaceIdentity {
                device: 4,
                inode: 111,
            },
            mount_namespace: NamespaceIdentity {
                device: 4,
                inode: 222,
            },
            passwd_sha256: "b".repeat(64),
            group_sha256: "c".repeat(64),
        };
        let host = AssertedWorkerHost {
            machine_id: policy.machine_id.clone(),
            boot_id: policy.boot_id.clone(),
            user_namespace: policy.user_namespace,
            mount_namespace: policy.mount_namespace,
        };
        let mut message = WORKER_POLICY_DOMAIN.to_vec();
        message.push(0);
        message.extend(line(&policy));
        let envelope = line(&SignedWorkerPolicy {
            schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
            policy: policy.clone(),
            signature_hex: crate::hex(&key.sign(&message).to_bytes()),
        });
        let trust =
            WorkerPolicyTrust::new(key.verifying_key(), &crate::sha256_hex(&line(&policy)), 7)
                .unwrap();
        let authority_root = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        fs::set_permissions(authority_root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let model = Self {
            root,
            authority_root,
            key,
            policy,
            host,
            envelope,
            trust,
        };
        fs::write(model.config(), model.document()).unwrap();
        fs::set_permissions(model.config(), fs::Permissions::from_mode(0o600)).unwrap();
        model
    }
    fn config(&self) -> PathBuf {
        self.root
            .path()
            .join("etc/hydracache-perf")
            .join(TRUST_FILE)
    }
    fn document(&self) -> Vec<u8> {
        line(&Document {
            schema_version: TRUST_INPUT_SCHEMA.into(),
            repository_id: crate::diagnostic_builder::REPOSITORY_ID,
            issuer_key_hex: crate::hex(&self.key.verifying_key().to_bytes()),
            worker_policy_body_sha256: crate::sha256_hex(&line(&self.policy)),
            policy_epoch: 7,
        })
    }
    fn input(&self) -> WorkerTrustInput {
        WorkerTrustInput {
            source: Source::open(self.root.path(), uid(), &self.key.verifying_key()).unwrap(),
            refused: false,
            _original_thread: PhantomData,
        }
    }
    fn authority(&self) -> FixedAuthorityRegistry {
        let policy =
            super::super::verify_worker_policy(&self.envelope, &self.trust, &self.host).unwrap();
        fixture::fixed::modeled_registry_for_test(
            fixture::initialize_fixture_registry(self.authority_root.path(), uid(), gid(), &policy)
                .unwrap(),
        )
    }
}

#[test]
fn worker_trust_input_strict_document_and_independent_issuer_epoch_pin() {
    let model = Model::new();
    let raw = model.document();
    assert!(decode(&raw, &model.key.verifying_key()).is_ok());
    assert!(decode(
        &raw,
        &crate::diagnostic_reviewed_trust::worker_issuer_key().unwrap()
    )
    .is_err());
    for field in [
        "schema_version",
        "repository_id",
        "issuer_key_hex",
        "worker_policy_body_sha256",
        "policy_epoch",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(decode(&line(&value), &model.key.verifying_key()).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        value[field] = serde_json::Value::Null;
        assert!(decode(&line(&value), &model.key.verifying_key()).is_err());
    }
    for changed in [
        vec![],
        vec![b' '; MAX_TRUST_BYTES + 1],
        raw[..raw.len() - 1].to_vec(),
        [raw.as_slice(), b"\n"].concat(),
        b"{\"policy_epoch\":7,\"policy_epoch\":7}\n".to_vec(),
    ] {
        assert!(decode(&changed, &model.key.verifying_key()).is_err());
    }
    for (field, value) in [
        ("policy_epoch", serde_json::json!(0)),
        ("repository_id", serde_json::json!(74)),
        ("schema_version", serde_json::json!("future")),
        ("extra", serde_json::json!(true)),
        ("issuer_key_hex", serde_json::json!("0".repeat(64))),
        (
            "worker_policy_body_sha256",
            serde_json::json!("Z".repeat(64)),
        ),
    ] {
        let mut body: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        body[field] = value;
        assert!(decode(&line(&body), &model.key.verifying_key()).is_err());
    }
}

#[test]
fn worker_trust_input_bounded_fragmented_short_and_interrupted_reads() {
    let bytes = b"fragmented";
    assert_eq!(
        bounded_read(|buffer, offset| {
            let offset = offset as usize;
            if offset == bytes.len() {
                return Ok(0);
            }
            buffer[0] = bytes[offset];
            Ok(1)
        })
        .unwrap(),
        bytes
    );
    let mut once = true;
    assert_eq!(
        bounded_read(|buffer, offset| {
            if once {
                once = false;
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            if offset > 0 {
                return Ok(0);
            }
            buffer[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        })
        .unwrap(),
        bytes
    );
    assert!(bounded_read(|_, _| Ok(0)).is_err());
    assert!(bounded_read(|_, _| Ok(usize::MAX)).is_err());
    assert!(bounded_read(|_, _| Err(std::io::ErrorKind::Interrupted.into())).is_err());
    assert!(bounded_read(|_, _| Err(std::io::ErrorKind::UnexpectedEof.into())).is_err());
    assert_eq!(
        bounded_read(|buffer, offset| {
            if offset == MAX_TRUST_BYTES as u64 {
                return Ok(0);
            }
            buffer[0] = 1;
            Ok(1)
        })
        .unwrap()
        .len(),
        MAX_TRUST_BYTES
    );
    assert!(bounded_read(|buffer, _| {
        buffer[0] = 1;
        Ok(1)
    })
    .is_err());
}

#[test]
fn worker_trust_input_model_policy_authority_bracket_retains_original_owners() {
    let model = Model::new();
    let mut source = model.input();
    let mut authority = model.authority();
    {
        let mut bound = source
            .verify_policy(&model.envelope, &model.host, &mut authority)
            .unwrap();
        assert_eq!(bound.policy_sha256(), model.trust.policy_sha256);
        bound.revalidate().unwrap();
        bound.revalidate().unwrap();
        assert!(!bound.is_refused());
    }
    assert!(!source.is_refused());
    assert!(!authority.is_failed());
    source
        .verify_policy(&model.envelope, &model.host, &mut authority)
        .unwrap()
        .revalidate()
        .unwrap();
}

#[test]
fn worker_trust_input_coarse_metadata_cannot_hide_changed_named_bytes() {
    let model = Model::new();
    let source = model.input();
    let mut named = Snapshot::new(
        OpenOptions::new().read(true).open(model.config()).unwrap(),
        uid(),
    )
    .unwrap();
    // Synthetic read-race/coarse-stamp counterexample: identical actual file stamps
    // with one changed byte in the private named read snapshot, not live root proof.
    named.raw[1] ^= 1;
    assert!(named.stamp == source.source.snapshot.stamp);
    assert!(source.source.snapshot.same(&named).is_err());
    let mut named = Snapshot::new(
        OpenOptions::new().read(true).open(model.config()).unwrap(),
        uid(),
    )
    .unwrap();
    named.raw.pop();
    assert!(source.source.snapshot.same(&named).is_err());
}

#[test]
fn worker_trust_input_source_drift_refuses_policy_and_authority_without_refresh() {
    for case in 0..6 {
        let model = Model::new();
        let mut source = model.input();
        let mut authority = model.authority();
        let mut bound = source
            .verify_policy(&model.envelope, &model.host, &mut authority)
            .unwrap();
        match case {
            0 => fs::write(model.config(), b"{}\n").unwrap(),
            1 => {
                let mut bytes = model.document();
                let index = bytes.iter().position(|&b| b == b'7').unwrap();
                bytes[index] = b'8';
                fs::write(model.config(), bytes).unwrap();
            }
            2 => {
                let old = model.config().with_extension("retained");
                fs::rename(model.config(), old).unwrap();
                fs::write(model.config(), model.document()).unwrap();
            }
            3 => fs::set_permissions(model.config(), fs::Permissions::from_mode(0o640)).unwrap(), // Safe but not original metadata.
            4 => fs::set_permissions(
                model.root.path().join("etc"),
                fs::Permissions::from_mode(0o777),
            )
            .unwrap(),
            _ => fs::rename(
                model.root.path().join("etc/hydracache-perf"),
                model.root.path().join("etc/retained"),
            )
            .unwrap(),
        }
        assert!(bound.revalidate().is_err());
        assert!(bound.is_refused());
        assert!(matches!(bound.revalidate(), Err(TrustInputError::Refused)));
        drop(bound);
        assert!(source.is_refused());
        assert!(authority.is_failed());
        if case < 4 {
            fs::write(model.config(), model.document()).unwrap();
            fs::set_permissions(model.config(), fs::Permissions::from_mode(0o600)).unwrap();
            assert!(matches!(
                source.verify_policy(&model.envelope, &model.host, &mut authority),
                Err(TrustInputError::Refused)
            ));
        }
    }
}

#[test]
fn worker_trust_input_constructor_mode_link_type_size_and_owner_refusals() {
    for case in 0..8 {
        let model = Model::new();
        match case {
            0 => fs::set_permissions(model.config(), fs::Permissions::from_mode(0o622)).unwrap(),
            1 => fs::set_permissions(model.config(), fs::Permissions::from_mode(0o700)).unwrap(),
            2 => fs::hard_link(model.config(), model.config().with_extension("alias")).unwrap(),
            3 => fs::write(model.config(), vec![b' '; MAX_TRUST_BYTES + 1]).unwrap(),
            4 => fs::write(model.config(), b"").unwrap(),
            5 => {
                fs::remove_file(model.config()).unwrap();
                fs::create_dir(model.config()).unwrap();
            }
            6 => {
                fs::set_permissions(model.root.path(), fs::Permissions::from_mode(0o770)).unwrap();
            }
            _ => {
                fs::remove_file(model.config()).unwrap();
                let name = CString::new(model.config().to_str().unwrap()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
        }
        assert!(Source::open(model.root.path(), uid(), &model.key.verifying_key()).is_err());
    }
    let model = Model::new();
    assert!(Source::open(model.root.path(), uid() + 1, &model.key.verifying_key()).is_err());
}

#[test]
fn worker_trust_input_named_root_ancestor_and_file_links_refuse() {
    for case in 0..3 {
        let model = Model::new();
        let path = match case {
            0 => model.root.path().join("etc"),
            1 => model.root.path().join("etc/hydracache-perf"),
            _ => model.config(),
        };
        let retained = path.with_extension("retained");
        fs::rename(&path, &retained).unwrap();
        std::os::unix::fs::symlink(&retained, &path).unwrap();
        assert!(Source::open(model.root.path(), uid(), &model.key.verifying_key()).is_err());
    }
    let model = Model::new();
    let source = model.input();
    let alias = model.root.path().with_extension("alias");
    std::os::unix::fs::symlink(model.root.path(), &alias).unwrap();
    assert!(Source::open(&alias, uid(), &model.key.verifying_key()).is_err());
    fs::remove_file(alias).unwrap();
    fs::set_permissions(model.root.path(), fs::Permissions::from_mode(0o750)).unwrap();
    assert!(source.check().is_err());
}

#[test]
fn worker_trust_input_foreign_policy_host_and_refused_authority_latch() {
    for case in 0..4 {
        let model = Model::new();
        let mut source = model.input();
        let mut authority = model.authority();
        let mut bytes = model.envelope.clone();
        let mut host = model.host.clone();
        match case {
            0 => bytes[0] = b'x',
            1 => host.boot_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into(),
            2 => {
                authority.refuse(RefusalReason::Operator).unwrap();
            }
            _ => {
                source.source.trust.epoch = 8;
            } // Wrong asserted input/journal join, not a live config update.
        }
        assert!(source.verify_policy(&bytes, &host, &mut authority).is_err());
        assert!(source.is_refused());
        assert!(authority.is_failed());
    }
}

#[test]
fn worker_trust_input_policy_owned_refusal_latches_both_originals() {
    let model = Model::new();
    let mut source = model.input();
    let mut authority = model.authority();
    let mut bound = source
        .verify_policy(&model.envelope, &model.host, &mut authority)
        .unwrap();
    bound.policy.refused = true;
    assert!(matches!(bound.revalidate(), Err(TrustInputError::Refused)));
    assert!(bound.input.is_refused());
    assert!(bound.authority.is_failed());
}

#[test]
fn worker_trust_input_seeded_document_bytes_and_independent_concurrency() {
    let model = Model::new();
    let raw = model.document();
    let mut state = 0x0751_2074u64;
    for _ in 0..256 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut changed = raw.clone();
        let offset = state as usize % changed.len();
        changed[offset] ^= 0x80;
        assert!(decode(&changed, &model.key.verifying_key()).is_err());
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let model = Model::new();
                    let mut source = model.input();
                    let mut authority = model.authority();
                    source
                        .verify_policy(&model.envelope, &model.host, &mut authority)
                        .unwrap()
                        .revalidate()
                        .unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    println!("seed=0x07512074 mutations=256 independent_guards=4 production_enrollment=false");
}

#[test]
fn worker_trust_input_fixed_constructor_refuses_nonroot_without_file_io() {
    let mut fixed = PathBuf::from("/");
    for name in ANCESTORS {
        fixed.push(name);
    }
    fixed.push(TRUST_FILE);
    assert_eq!(fixed, Path::new(FIXED_TRUST_PATH));
    assert_ne!(
        uid(),
        0,
        "local proof requires nonroot; never inspect a production file from this test"
    );
    assert!(matches!(
        open_fixed_worker_trust(),
        Err(TrustInputError::Invalid)
    ));
}

fn kernel_local_host() -> AssertedWorkerHost {
    let namespace = |name: &str| {
        let m = File::open(format!("/proc/thread-self/ns/{name}"))
            .unwrap()
            .metadata()
            .unwrap();
        NamespaceIdentity {
            device: m.dev(),
            inode: m.ino(),
        }
    };
    AssertedWorkerHost {
        machine_id: fs::read_to_string("/etc/machine-id")
            .unwrap()
            .trim_end_matches('\n')
            .into(),
        boot_id: fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .unwrap()
            .trim_end_matches('\n')
            .into(),
        user_namespace: namespace("user"),
        mount_namespace: namespace("mnt"),
    }
}
fn kernel_model(host: AssertedWorkerHost) -> Model {
    let mut model = Model::new();
    model.host = host;
    model.policy.machine_id = model.host.machine_id.clone();
    model.policy.boot_id = model.host.boot_id.clone();
    model.policy.user_namespace = model.host.user_namespace;
    model.policy.mount_namespace = model.host.mount_namespace;
    let body = line(&model.policy);
    model.trust =
        WorkerPolicyTrust::new(model.key.verifying_key(), &crate::sha256_hex(&body), 7).unwrap();
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(body);
    model.envelope = line(&SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy: model.policy.clone(),
        signature_hex: crate::hex(&model.key.sign(&message).to_bytes()),
    });
    fs::write(model.config(), model.document()).unwrap();
    model
}
fn kernel_packet() -> crate::diagnostic_kernel_admission::KernelPacket {
    kernel_packet_bytes(b"context-observation-only-not-authorized-request")
}
pub(super) fn kernel_packet_bytes(
    bytes: &[u8],
) -> crate::diagnostic_kernel_admission::KernelPacket {
    use std::os::fd::{FromRawFd, OwnedFd};
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    let receiver = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let sender = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let mut receiver = crate::diagnostic_kernel_admission::KernelReceiver::new(receiver).unwrap();
    assert_eq!(
        unsafe {
            libc::send(
                sender.as_raw_fd(),
                bytes.as_ptr().cast(),
                bytes.len(),
                libc::MSG_NOSIGNAL,
            )
        },
        bytes.len() as isize
    );
    receiver.try_receive().unwrap().unwrap()
}

pub(super) fn controller_worker_fixture() -> (
    TempDir,
    TempDir,
    WorkerTrustInput,
    FixedAuthorityRegistry,
    Vec<u8>,
) {
    let model = kernel_model(kernel_local_host());
    let input = model.input();
    let authority = model.authority();
    (
        model.root,
        model.authority_root,
        input,
        authority,
        model.envelope,
    )
}

// Test-only parent-owned authority origin: retain the strict /tmp leaf rule.
pub(super) fn controller_worker_fixture_at(
    authority_root: &std::path::Path,
) -> (TempDir, WorkerTrustInput, FixedAuthorityRegistry, Vec<u8>) {
    let model = kernel_model(kernel_local_host());
    let input = model.input();
    let checked =
        super::super::verify_worker_policy(&model.envelope, &model.trust, &model.host).unwrap();
    let authority = fixture::fixed::modeled_registry_for_test(
        fixture::initialize_fixture_registry(authority_root, uid(), gid(), &checked).unwrap(),
    );
    // The unused empty authority TempDir is dropped here, before any checkpoint.
    (model.root, input, authority, model.envelope)
}

#[test]
fn worker_kernel_binding_actual_context_roundtrip_and_healthy_drop() {
    let model = kernel_model(kernel_local_host());
    let mut input = model.input();
    let mut authority = model.authority();
    let mut packet = kernel_packet();
    {
        let mut binding = input
            .verify_kernel_policy(&model.envelope, &mut packet, &mut authority)
            .unwrap();
        for _ in 0..3 {
            binding.revalidate().unwrap();
        }
        assert!(!binding.is_refused());
    }
    assert!(!input.is_refused() && !authority.is_failed() && !packet.is_refused());
    packet.revalidate_sender().unwrap();
    input
        .verify_kernel_policy(&model.envelope, &mut packet, &mut authority)
        .unwrap()
        .revalidate()
        .unwrap();
}
#[test]
fn worker_kernel_binding_signed_wrong_machine_boot_and_namespaces_refuse_all() {
    for case in 0..4 {
        let mut host = kernel_local_host();
        match case {
            0 => {
                host.machine_id = if host.machine_id == "a".repeat(32) {
                    "b".repeat(32)
                } else {
                    "a".repeat(32)
                }
            }
            1 => host.boot_id = "11111111-2222-3333-4444-555555555555".into(),
            2 => host.user_namespace.inode += 1,
            _ => host.mount_namespace.inode += 1,
        }
        let model = kernel_model(host);
        let mut input = model.input();
        let mut authority = model.authority();
        let mut packet = kernel_packet();
        assert!(input
            .verify_kernel_policy(&model.envelope, &mut packet, &mut authority)
            .is_err());
        assert!(input.is_refused() && authority.is_failed() && packet.is_refused());
        assert!(matches!(
            input.verify_kernel_policy(&model.envelope, &mut packet, &mut authority),
            Err(TrustInputError::Refused)
        ));
    }
}
#[test]
fn worker_kernel_binding_source_packet_probe_and_policy_failures_cross_latch() {
    for case in 0..4 {
        let model = kernel_model(kernel_local_host());
        let mut input = model.input();
        let mut authority = model.authority();
        let mut packet = kernel_packet();
        {
            let mut binding = input
                .verify_kernel_policy(&model.envelope, &mut packet, &mut authority)
                .unwrap();
            if case == 0 {
                fs::write(model.config(), b"changed\n").unwrap();
            } else {
                binding.simulate_owned_fault_for_test(case - 1);
            }
            assert!(binding.revalidate().is_err());
            fs::write(model.config(), model.document()).unwrap();
            assert!(binding.is_refused());
            assert!(matches!(
                binding.revalidate(),
                Err(TrustInputError::Refused)
            ));
        }
        assert!(input.is_refused() && authority.is_failed() && packet.is_refused());
    }
}
#[test]
fn worker_kernel_binding_malformed_prior_refusal_and_wrong_envelope_never_recover() {
    for case in 0..6 {
        let model = kernel_model(kernel_local_host());
        let mut input = model.input();
        let mut authority = model.authority();
        let mut packet = kernel_packet();
        let mut bad = model.envelope.clone();
        match case {
            0 => bad = b"{}\n".to_vec(),
            1 => bad = vec![b'x'; super::super::MAX_SIGNED_WORKER_POLICY_BYTES + 1],
            2 => {
                let mut value: serde_json::Value = serde_json::from_slice(&bad).unwrap();
                value["signature_hex"] = "0".repeat(128).into();
                bad = line(&value);
            }
            3 => packet.latch_refusal(),
            4 => authority.latch_refusal(),
            _ => input.refused = true,
        }
        assert!(input
            .verify_kernel_policy(&bad, &mut packet, &mut authority)
            .is_err());
        assert!(input.is_refused() && authority.is_failed() && packet.is_refused());
    }
}
#[test]
fn worker_kernel_binding_seeded_context_mutations_and_independent_threads() {
    let mut seed = 0x07522074_u64;
    for _ in 0..32 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut host = kernel_local_host();
        host.user_namespace.inode = host.user_namespace.inode.wrapping_add(1 + seed % 1000);
        let model = kernel_model(host);
        let mut input = model.input();
        let mut authority = model.authority();
        let mut packet = kernel_packet();
        assert!(input
            .verify_kernel_policy(&model.envelope, &mut packet, &mut authority)
            .is_err());
        assert!(input.is_refused() && authority.is_failed() && packet.is_refused());
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let model = kernel_model(kernel_local_host());
                    let mut input = model.input();
                    let mut authority = model.authority();
                    let mut packet = kernel_packet();
                    input
                        .verify_kernel_policy(&model.envelope, &mut packet, &mut authority)
                        .unwrap()
                        .revalidate()
                        .unwrap();
                    assert!(!input.is_refused() && !authority.is_failed() && !packet.is_refused());
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    println!("seed=0x07522074 signed_context_mutations=32 independent_threads=4 production_enrollment=false");
}
