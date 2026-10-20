//! Actual kernel original and sealed FDs, not a privileged FixedPlacement fixture.
use super::*;
use crate::diagnostic_builder::held_build::role_files::PinnedRoles;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd};

fn raw_elf(tag: u8) -> Vec<u8> {
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
fn build_log(extra: bool) -> Vec<u8> {
    let mut log = Vec::new();
    if extra {
        log.extend(line(&json!({"reason":"build-script-executed"})));
    }
    for (name, source) in [
        (held::HOST, "main.rs"),
        (held::LAUNCHER, "bin/diagnostic_held_launcher.rs"),
    ] {
        log.extend(line(&json!({"reason":"compiler-artifact","package_id":"path+file:///isolated/tools/long-run-supervisor-074#0.0.0","target":{"name":name,"kind":["bin"],"crate_types":["bin"],"src_path":format!("/isolated/tools/long-run-supervisor-074/src/{source}")},"profile":{"opt_level":"3","debug_assertions":false,"test":false},"features":[],"executable":format!("/target/x86_64-unknown-linux-gnu/release/{name}"),"fresh":false})));
    }
    log.extend(line(&json!({"reason":"build-finished","success":true})));
    log
}
fn policy(f: &Fixture) -> held::CheckedPolicy {
    held::load_policy(
        &f.held_policy,
        &f.document.held_policy_sha256,
        &held::ExpectedTrust {
            controller_key: &f.selection.controller,
            authority_epoch: f.document.policy_epoch,
            worker_policy: &f.worker_policy,
            worker_policy_sha256: &f.selection.worker_builder_policy_sha256,
        },
    )
    .unwrap()
}
fn sealed(raw: &[u8]) -> File {
    let fd = unsafe {
        libc::memfd_create(
            c"hc074-consuming-held".as_ptr(),
            libc::MFD_ALLOW_SEALING | libc::MFD_CLOEXEC,
        )
    };
    assert!(fd >= 0);
    let mut writer = unsafe { File::from_raw_fd(fd) };
    writer.write_all(raw).unwrap();
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
fn roles(f: &Fixture, receipt: &[u8]) -> (PinnedRoles, File) {
    let build = held::verify_receipt(receipt, &policy(f)).unwrap();
    let host = sealed(&raw_elf(1));
    let alias = host.try_clone().unwrap();
    let launcher = sealed(&raw_elf(2));
    let artifacts = crate::diagnostic_two_phase_launch::LaunchArtifacts {
        launcher_sha256: crate::sha256_hex(&raw_elf(2)),
        watchdog_sha256: crate::sha256_hex(&raw_elf(1)),
        collector_sha256: crate::sha256_hex(&raw_elf(1)),
    };
    (
        build
            .pin_sealed_roles(
                host,
                launcher,
                b"synthetic-lock\n",
                &build_log(false),
                &artifacts,
            )
            .unwrap(),
        alias,
    )
}
fn placement(raw: &[u8]) -> Vec<u8> {
    let doc: role_binding::SignedBinding = serde_json::from_slice(raw).unwrap();
    line(&doc.binding.placement)
}
#[test]
fn consuming_held_actual_kernel_selection_receipt_and_retained_memfds_are_rechecked() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let v3 = placement(&raw);
        let (mut roles, _alias) = roles(f, &receipt);
        let mut source = f.input();
        let first = source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &v3,
            )
            .unwrap();
        first.revalidate_roles(&mut roles).unwrap();
        let closing = source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &v3,
            )
            .unwrap();
        assert!(closing.same_original(&first));
        closing.revalidate_roles(&mut roles).unwrap();
        assert!(!original.is_refused());
    });
}
#[test]
fn consuming_held_exact_original_v3_bytes_are_required_even_for_valid_v4() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let mut v3 = placement(&raw);
        v3.push(b'\n');
        let mut source = f.input();
        assert!(source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &v3
            )
            .is_err());
        assert!(source.is_refused() && original.is_refused());
    });
}
#[test]
fn consuming_held_metadata_drift_on_real_retained_memfd_refuses_stickily() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let (mut roles, alias) = roles(f, &receipt);
        let mut source = f.input();
        let check = source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &placement(&raw),
            )
            .unwrap();
        check.revalidate_roles(&mut roles).unwrap();
        assert_eq!(unsafe { libc::fchmod(alias.as_raw_fd(), 0o600) }, 0);
        assert!(check.revalidate_roles(&mut roles).is_err());
        assert!(check.revalidate_roles(&mut roles).is_err());
    });
}
#[test]
fn consuming_held_same_images_with_another_valid_receipt_statement_refuse() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let (mut roles, _alias) = roles(f, &receipt);
        let mut doc: role_binding::SignedBinding = serde_json::from_slice(&raw).unwrap();
        let old: held::SignedReceipt = serde_json::from_slice(&receipt).unwrap();
        let host = raw_elf(1);
        let launcher = raw_elf(2);
        let log = build_log(true);
        let checked = policy(f);
        let different = line(
            &held::sign_build(
                &checked,
                &SigningKey::from_bytes(&[82; 32]),
                &old.statement.observation,
                &held::Contents {
                    original_host: &host,
                    held_launcher: &launcher,
                    root_lock: b"synthetic-lock\n",
                    build_log: &log,
                },
            )
            .unwrap(),
        );
        assert_ne!(receipt, different);
        doc.binding.held_receipt_sha256 = crate::sha256_hex(&different);
        doc.signature_hex = crate::hex(
            &SigningKey::from_bytes(&[76; 32])
                .sign(&role_binding::signing_message(&doc.binding, &checked, &different).unwrap())
                .to_bytes(),
        );
        let raw = line(&doc);
        let mut source = f.input();
        let check = source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &different, &raw],
                &placement(&raw),
            )
            .unwrap();
        assert!(check.revalidate_roles(&mut roles).is_err());
        assert!(check.revalidate_roles(&mut roles).is_err());
    });
}
#[test]
fn consuming_held_actual_source_replacement_poison_cannot_be_repaired() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let mut source = f.input();
        let v3 = placement(&raw);
        source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &v3,
            )
            .unwrap();
        let saved = f.root.path().join("saved");
        fs::rename(f.path(), saved).unwrap();
        f.write();
        assert!(source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &v3
            )
            .is_err());
        assert!(source.is_refused() && original.is_refused());
    });
}
#[test]
fn consuming_held_private_retirement_closes_source_without_exporting_the_original() {
    with_original(|original, f| {
        let (raw, receipt) = binding_pair(f, &original.request, 0);
        let mut source = f.input();
        source.refuse_late_join();
        assert!(source
            .inspect_late_join(
                original,
                [&f.held_policy, &f.worker_policy, &receipt, &raw],
                &placement(&raw)
            )
            .is_err());
        assert!(source.is_refused() && original.is_refused());
    });
}
