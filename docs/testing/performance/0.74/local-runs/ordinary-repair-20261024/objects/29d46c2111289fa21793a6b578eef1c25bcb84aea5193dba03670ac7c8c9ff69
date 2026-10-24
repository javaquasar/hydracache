//! Isolated fresh requests, not full ActiveAuthority, syscall latency or admission.
use super::*;
use crate::diagnostic_worker_policy::{authority_registry, *};
use ed25519_dalek::{Signer, SigningKey};
use std::os::unix::fs::PermissionsExt;

const ITERATIONS: usize = 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
struct Io {
    rchar: u64,
    wchar: u64,
    syscr: u64,
    syscw: u64,
}
impl Io {
    fn parse(raw: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(raw).ok()?;
        let mut fields = [None; 4];
        for line in text.lines() {
            let (name, value) = line.split_once(':')?;
            if let Some(index) = ["rchar", "wchar", "syscr", "syscw"]
                .iter()
                .position(|expected| *expected == name)
            {
                if fields[index]
                    .replace(value.trim().parse::<u64>().ok()?)
                    .is_some()
                {
                    return None;
                }
            }
        }
        Some(Self {
            rchar: fields[0]?,
            wchar: fields[1]?,
            syscr: fields[2]?,
            syscw: fields[3]?,
        })
    }
    fn read(file: &File) -> Self {
        let mut raw = [0; 1024];
        let count = file.read_at(&mut raw, 0).unwrap();
        assert!(count < raw.len());
        Self::parse(&raw[..count]).unwrap()
    }
    fn since(self, before: Self) -> Self {
        Self {
            rchar: self.rchar.checked_sub(before.rchar).unwrap(),
            wchar: self.wchar.checked_sub(before.wchar).unwrap(),
            syscr: self.syscr.checked_sub(before.syscr).unwrap(),
            syscw: self.syscw.checked_sub(before.syscw).unwrap(),
        }
    }
}
struct Journal {
    // Drop the locked owner before removing its isolated tree.
    registry: authority_registry::fixture::FixtureAuthorityRegistry,
    trust: WorkerPolicyTrust,
    host: AssertedWorkerHost,
    root: tempfile::TempDir,
}
fn policy(epoch: u64) -> (CheckedWorkerPolicy, WorkerPolicyTrust, AssertedWorkerHost) {
    let host = AssertedWorkerHost {
        machine_id: "a".repeat(32),
        boot_id: "00000000-0000-0000-0000-000000000001".into(),
        user_namespace: NamespaceIdentity {
            device: 1,
            inode: 2,
        },
        mount_namespace: NamespaceIdentity {
            device: 1,
            inode: 3,
        },
    };
    let policy = WorkerPolicy {
        schema_version: WORKER_POLICY_SCHEMA.into(),
        repository_id: 1_217_101_761,
        purpose: "diagnostic-worker-only".into(),
        account_source: "local-files".into(),
        account: "hydracache-perf".into(),
        group: "hydracache-perf".into(),
        policy_epoch: epoch,
        uid: 1000,
        gid: 1000,
        supplementary_gids: vec![1000],
        machine_id: host.machine_id.clone(),
        boot_id: host.boot_id.clone(),
        user_namespace: host.user_namespace,
        mount_namespace: host.mount_namespace,
        passwd_sha256: "b".repeat(64),
        group_sha256: "c".repeat(64),
    };
    let key = SigningKey::from_bytes(&[74; 32]); // Synthetic fixture issuer only.
    let mut body = crate::canonical_json(&policy).unwrap();
    body.push(b'\n');
    let trust =
        WorkerPolicyTrust::new(key.verifying_key(), &crate::sha256_hex(&body), epoch).unwrap();
    let mut message = WORKER_POLICY_DOMAIN.to_vec();
    message.push(0);
    message.extend(body);
    let mut bytes = crate::canonical_json(&SignedWorkerPolicy {
        schema_version: SIGNED_WORKER_POLICY_SCHEMA.into(),
        policy,
        signature_hex: crate::hex(&key.sign(&message).to_bytes()),
    })
    .unwrap();
    bytes.push(b'\n');
    (
        verify_worker_policy(&bytes, &trust, &host).unwrap(),
        trust,
        host,
    )
}
impl Journal {
    fn new(generations: usize) -> Self {
        assert!(matches!(generations, 1 | 7));
        assert_ne!(unsafe { libc::geteuid() }, 0);
        let root = tempfile::Builder::new()
            .prefix("hydracache-authority-")
            .tempdir_in("/tmp")
            .unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let (checked, mut trust, host) = policy(7);
        let mut registry = authority_registry::fixture::initialize_fixture_registry(
            root.path(),
            unsafe { libc::geteuid() },
            unsafe { libc::getegid() },
            &checked,
        )
        .unwrap();
        for epoch in 8..8 + (generations / 2) as u64 {
            registry
                .refuse(authority_registry::RefusalReason::Operator)
                .unwrap();
            let (checked, next, _) = policy(epoch);
            registry.advance(&checked).unwrap();
            trust = next;
        }
        let result = Self {
            registry,
            trust,
            host,
            root,
        };
        assert_eq!(result.registry.view().unwrap().revision, generations as u64);
        result
    }
    fn inspect(&mut self) {
        let observed = self.registry.inspect(&self.trust, &self.host).unwrap();
        assert!(!observed.refused);
        std::hint::black_box(observed);
    }
}

#[test]
fn validation_leaf_requests_io_parser_refuses_missing_duplicate_overflow_and_bad_text() {
    let valid = b"rchar: 1\nwchar: 2\nsyscr: 3\nsyscw: 4\nread_bytes: 0\n";
    assert_eq!(
        Io::parse(valid),
        Some(Io {
            rchar: 1,
            wchar: 2,
            syscr: 3,
            syscw: 4
        })
    );
    for raw in [
        b"".as_slice(),
        b"rchar: 1\nwchar: 2\nsyscr: 3\n",
        b"rchar: 1\nwchar: 2\nsyscr: 3\nsyscw: 4\nsyscr: 5\n",
        b"rchar: 18446744073709551616\nwchar: 2\nsyscr: 3\nsyscw: 4\n",
        b"rchar: -1\nwchar: 2\nsyscr: 3\nsyscw: 4\n",
        b"\xff",
    ] {
        assert!(Io::parse(raw).is_none());
    }
}
#[test]
fn validation_leaf_requests_retained_io_reader_observes_reads_without_writes() {
    let file = File::open("/proc/thread-self/io").unwrap();
    let before = Io::read(&file);
    let after = Io::read(&file).since(before);
    assert_eq!(after.syscr, 1);
    assert_eq!(after.syscw, 0);
    assert!(after.rchar > 0);
    assert_eq!(after.wchar, 0);
}
#[test]
fn validation_leaf_requests_authority_drift_is_sticky_and_never_reinitializes() {
    let mut journal = Journal::new(7);
    journal.inspect();
    std::fs::remove_file(journal.root.path().join("0000000000000007.commit")).unwrap();
    assert!(journal
        .registry
        .inspect(&journal.trust, &journal.host)
        .is_err());
    assert!(journal.registry.is_failed());
    assert!(matches!(
        journal.registry.inspect(&journal.trust, &journal.host),
        Err(authority_registry::AuthorityError::Refused)
    ));
    assert!(!journal.root.path().join("0000000000000007.commit").exists());
}
#[test]
#[ignore = "manual local nonroot allocation and proc I/O request controls; not admission or timing"]
fn validation_leaf_requests_attribution() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let origin = Origin::open().unwrap();
    let mut one = Journal::new(1);
    let mut seven = Journal::new(7);
    let io = File::open("/proc/thread-self/io").unwrap();
    println!(
        "HC074-VALIDATION-LEAF-ORIGIN {}",
        serde_json::json!({
            "pid":std::process::id(),"boot_id":origin.boot,"uid":unsafe { libc::geteuid() },
            "iterations":ITERATIONS,"samples":3,"warmups":32,"server":false,"product":false,
        })
    );
    for case in [
        "boot",
        "namespace-user",
        "namespace-mnt",
        "namespace-pid",
        "namespace-time",
        "origin",
        "authority-1",
        "authority-7",
    ] {
        let mut run = |iterations| {
            for _ in 0..iterations {
                match case {
                    "boot" => assert!(
                        crate::diagnostic_kernel_admission::origin::boot_comparison::matches(
                            &origin.root,
                            &origin.boot
                        )
                        .unwrap()
                    ),
                    "origin" => origin.revalidate().unwrap(),
                    "authority-1" => one.inspect(),
                    "authority-7" => seven.inspect(),
                    _ => {
                        std::hint::black_box(
                            namespace(case.strip_prefix("namespace-").unwrap()).unwrap(),
                        );
                    }
                }
            }
        };
        run(32);
        let ((), trace) = crate::diagnostic_ipc::validation_trace::kernel_walk::capture(|| run(64));
        let expected = match case {
            "boot" => [0, 0, 0, 0, 0, 64, 0, 0, 0, 64],
            "origin" => [0, 0, 0, 0, 64, 128, 256, 0, 0, 128],
            "authority-1" | "authority-7" => [0; 10],
            _ => [0, 0, 0, 0, 0, 0, 64, 0, 0, 0],
        };
        assert_eq!(trace.nodes.map(|s| s.calls), expected);
        for sample in 0..3 {
            let ((), counts) = capture(|| run(ITERATIONS));
            println!(
                "HC074-VALIDATION-LEAF-ALLOC {}",
                serde_json::json!({
                    "case":case,"sample":sample,"iterations":ITERATIONS,"counts":counts,
                    "rust_requests_only":true,"authority_fixed_ancestry":false,"server":false,"product":false,
                })
            );
            let before = Io::read(&io);
            let empty = Io::read(&io).since(before);
            let before = Io::read(&io);
            run(ITERATIONS);
            let observed = Io::read(&io).since(before);
            assert_eq!(empty.syscr, 1);
            assert_eq!(empty.syscw, 0);
            assert_eq!(observed.syscw, 0);
            assert_eq!(observed.wchar, 0);
            println!(
                "HC074-VALIDATION-LEAF-IO {}",
                serde_json::json!({
                    "case":case,"sample":sample,"iterations":ITERATIONS,"empty":empty,"observed":observed,
                    "kernel_calls":expected,"kernel_iterations":64,"allocator_active":false,
                    "all_syscalls_observed":false,"authority_fixed_ancestry":false,"server":false,"product":false,
                })
            );
        }
    }
    origin.revalidate().unwrap();
    one.inspect();
    seven.inspect();
}
