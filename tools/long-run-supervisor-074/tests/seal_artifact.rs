use hydracache_long_run_supervisor_074::archive::{
    create_deterministic_archive, ArchiveLimits, ARCHIVE_NAME,
};
use hydracache_long_run_supervisor_074::artifact::{
    build_packet, GuardEvidenceInput, PacketLimits, PacketPlan, PacketResult, RoleEvidenceInput,
};
use hydracache_long_run_supervisor_074::seal_artifact::{
    build_or_recover_seal_artifact, SealArtifactError,
};
use hydracache_long_run_supervisor_074::{
    append_record, build_record, CheckpointPayload, Phase, ProcessIdentity, Role, GENESIS_HASH,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn process(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        boot_id: "boot-a".to_owned(),
        pid,
        start_ticks: u64::from(pid) * 100,
        process_group: 10,
        cgroup_path: "/hc/a".to_owned(),
        cgroup_inode: 50,
        unit_name: "hydracache-performance-074-a.service".to_owned(),
    }
}

fn payload(sequence: u64) -> CheckpointPayload {
    CheckpointPayload {
        campaign_id: "a".repeat(64),
        role: Role::I74,
        phase: if sequence == 2 {
            Phase::Terminal
        } else {
            Phase::Measured
        },
        phase_epoch: 1,
        monotonic_elapsed_ns: sequence * 1_000,
        wall_clock_utc: format!("2026-10-04T00:00:{sequence:02}Z"),
        completed: sequence,
        failed: 0,
        rejected: 0,
        timed_out: 0,
        outstanding: 0,
        telemetry_sequence: sequence,
        milestone: if sequence == 2 {
            "terminal"
        } else {
            "measured"
        }
        .to_owned(),
        surface_counters: BTreeMap::from([("resp".to_owned(), sequence)]),
        resource_counters: BTreeMap::new(),
        owner_counters: BTreeMap::new(),
        harness: process(100),
        daemon: process(101),
    }
}

fn limits() -> (PacketLimits, ArchiveLimits) {
    (
        PacketLimits {
            maximum_files: 2_000,
            maximum_bytes: 1_073_741_824,
        },
        ArchiveLimits {
            maximum_files: 2_002,
            maximum_uncompressed_bytes: 1_073_741_824,
            maximum_archive_bytes: 1_073_741_824,
        },
    )
}

fn fixture(root: &Path) -> (PathBuf, PathBuf, PacketPlan) {
    let campaign_id = "a".repeat(64);
    let campaign = root.join(&campaign_id);
    let role = campaign.join("roles/i74");
    fs::create_dir_all(role.join("guards")).unwrap();
    let manifest = serde_json::to_vec(&serde_json::json!({
        "campaign_id": campaign_id,
        "release": "0.74"
    }))
    .unwrap();
    fs::write(campaign.join("campaign-start.json"), &manifest).unwrap();
    fs::write(
        role.join("guards/semantic.json"),
        serde_json::to_vec(&serde_json::json!({"id": "semantic", "passed": true})).unwrap(),
    )
    .unwrap();
    let journal = role.join("checkpoints.jsonl");
    let head = role.join("checkpoints.head");
    let first = build_record(1, GENESIS_HASH, payload(1)).unwrap();
    append_record(&journal, &head, &first).unwrap();
    let second = build_record(2, &first.record_sha256, payload(2)).unwrap();
    append_record(&journal, &head, &second).unwrap();
    let seal_root = root.join("seals");
    fs::create_dir(&seal_root).unwrap();
    let plan = PacketPlan {
        campaign_id: "a".repeat(64),
        campaign_manifest_sha256: hex(&Sha256::digest(&manifest)),
        continuation_packet_sha256: None,
        result: PacketResult::Complete,
        promotable: false,
        terminal_reason: None,
        required_final_guards: vec!["semantic".to_owned()],
        guard_evidence: vec![GuardEvidenceInput {
            id: "semantic".to_owned(),
            passed: true,
            source_relative_path: PathBuf::from("roles/i74/guards/semantic.json"),
        }],
        roles: vec![RoleEvidenceInput {
            role: Role::I74,
            result: PacketResult::Complete,
            journal_relative_path: PathBuf::from("roles/i74/checkpoints.jsonl"),
        }],
        raw_files: vec![
            PathBuf::from("campaign-start.json"),
            PathBuf::from("roles/i74/checkpoints.jsonl"),
            PathBuf::from("roles/i74/guards/semantic.json"),
        ],
    };
    (campaign, seal_root, plan)
}

#[test]
fn durable_seal_artifact_is_exactly_replayed_and_tamper_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let (campaign, seal_root, plan) = fixture(temporary.path());
    let (packet_limits, archive_limits) = limits();
    let first = build_or_recover_seal_artifact(
        &campaign,
        &seal_root,
        "123e4567-e89b-42d3-a456-426614174000",
        &"b".repeat(64),
        Role::I74,
        &plan,
        packet_limits,
        archive_limits,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let packet = seal_root.join(&first.packet_directory_name);
        let archive = seal_root.join(&first.archive_directory_name);
        assert_eq!(
            fs::metadata(&packet).unwrap().permissions().mode() & 0o777,
            0o500
        );
        assert_eq!(
            fs::metadata(packet.join("packet-manifest.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o400
        );
        assert_eq!(
            fs::metadata(&archive).unwrap().permissions().mode() & 0o777,
            0o500
        );
    }
    fs::remove_file(campaign.join("i74-seal-intent.json.sha256")).unwrap();
    fs::remove_file(campaign.join("i74-seal-result.json.sha256")).unwrap();
    let replay = build_or_recover_seal_artifact(
        &campaign,
        &seal_root,
        "123e4567-e89b-42d3-a456-426614174000",
        &"b".repeat(64),
        Role::I74,
        &plan,
        packet_limits,
        archive_limits,
    )
    .unwrap();
    assert_eq!(replay, first);
    for name in [
        "i74-seal-intent.json",
        "i74-seal-intent.json.sha256",
        "i74-seal-result.json",
        "i74-seal-result.json.sha256",
    ] {
        assert!(campaign.join(name).is_file(), "missing {name}");
    }

    let archive_path = seal_root
        .join(&first.archive_directory_name)
        .join(ARCHIVE_NAME);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&archive_path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    fs::write(&archive_path, b"tampered").unwrap();
    assert!(build_or_recover_seal_artifact(
        &campaign,
        &seal_root,
        "123e4567-e89b-42d3-a456-426614174000",
        &"b".repeat(64),
        Role::I74,
        &plan,
        packet_limits,
        archive_limits,
    )
    .is_err());
}

#[test]
fn completed_packet_and_archive_without_result_are_adopted_without_rebuild() {
    let temporary = tempfile::tempdir().unwrap();
    let (campaign, seal_root, plan) = fixture(temporary.path());
    let (packet_limits, archive_limits) = limits();
    let base = format!("{}-i74", plan.campaign_id);
    let packet_directory = seal_root.join(format!("{base}-packet"));
    let packet = build_packet(&campaign, &packet_directory, &plan, packet_limits).unwrap();
    let archive_directory = seal_root.join(format!("{base}-archive"));
    let archive =
        create_deterministic_archive(&packet.packet_directory, &archive_directory, archive_limits)
            .unwrap();

    let recovered = build_or_recover_seal_artifact(
        &campaign,
        &seal_root,
        "223e4567-e89b-42d3-a456-426614174000",
        &"c".repeat(64),
        Role::I74,
        &plan,
        packet_limits,
        archive_limits,
    )
    .unwrap();
    assert_eq!(
        recovered.packet_manifest_sha256,
        packet.packet_manifest_sha256
    );
    assert_eq!(recovered.archive_sha256, archive.archive_sha256);
    assert!(campaign.join("i74-seal-result.json").is_file());
}

#[test]
fn complete_building_directories_are_recovered_but_conflicting_intent_is_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let (campaign, seal_root, plan) = fixture(temporary.path());
    let (packet_limits, archive_limits) = limits();
    let base = format!("{}-i74", plan.campaign_id);
    let packet_final = seal_root.join(format!("{base}-packet"));
    build_packet(&campaign, &packet_final, &plan, packet_limits).unwrap();
    let packet_stage = seal_root.join(format!(".{base}-packet.building"));
    fs::rename(&packet_final, &packet_stage).unwrap();
    let archive_final = seal_root.join(format!("{base}-archive"));
    create_deterministic_archive(&packet_stage, &archive_final, archive_limits).unwrap();
    let archive_stage = seal_root.join(format!(".{base}-archive.building"));
    fs::rename(&archive_final, &archive_stage).unwrap();

    build_or_recover_seal_artifact(
        &campaign,
        &seal_root,
        "323e4567-e89b-42d3-a456-426614174000",
        &"d".repeat(64),
        Role::I74,
        &plan,
        packet_limits,
        archive_limits,
    )
    .unwrap();
    assert!(packet_final.is_dir());
    assert!(archive_final.is_dir());
    assert!(!packet_stage.exists());
    assert!(!archive_stage.exists());

    assert!(matches!(
        build_or_recover_seal_artifact(
            &campaign,
            &seal_root,
            "423e4567-e89b-42d3-a456-426614174000",
            &"d".repeat(64),
            Role::I74,
            &plan,
            packet_limits,
            archive_limits,
        ),
        Err(SealArtifactError::Binding)
    ));
}
