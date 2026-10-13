//! Modeled test-only context inputs, never production enrollment or reboot.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InspectionProfile {
    Same,
    Boot,
    Machine,
    UserNamespace,
    MountNamespace,
    Digest,
    Epoch,
    Issuer,
}
pub(super) const PROFILES: [InspectionProfile; 8] = [
    InspectionProfile::Same,
    InspectionProfile::Boot,
    InspectionProfile::Machine,
    InspectionProfile::UserNamespace,
    InspectionProfile::MountNamespace,
    InspectionProfile::Digest,
    InspectionProfile::Epoch,
    InspectionProfile::Issuer,
];
impl InspectionProfile {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Same => "same",
            Self::Boot => "boot",
            Self::Machine => "machine",
            Self::UserNamespace => "user-namespace",
            Self::MountNamespace => "mount-namespace",
            Self::Digest => "digest",
            Self::Epoch => "epoch",
            Self::Issuer => "issuer",
        }
    }
    pub(super) fn parse(name: &str) -> Option<Self> {
        PROFILES.into_iter().find(|profile| profile.name() == name)
    }
}
fn different_hex(original: &str) -> String {
    assert!(!original.is_empty());
    let mut value = original.to_owned();
    value.replace_range(..1, if original.starts_with('a') { "b" } else { "a" });
    value
}
fn different_inode(inode: u64) -> u64 {
    assert!(inode > 0);
    inode.checked_add(1).unwrap_or(inode - 1)
}
pub(super) fn inputs(
    original: &AssertedWorkerHost,
    pin: &str,
    profile: InspectionProfile,
) -> (AssertedWorkerHost, WorkerPolicyTrust) {
    assert!(crate::is_hash(pin));
    let mut host = original.clone();
    let mut digest = pin.to_owned();
    let mut epoch = 7;
    let mut issuer = 74;
    match profile {
        InspectionProfile::Same => {}
        InspectionProfile::Boot => host.boot_id = different_hex(&host.boot_id),
        InspectionProfile::Machine => host.machine_id = different_hex(&host.machine_id),
        InspectionProfile::UserNamespace => {
            host.user_namespace.inode = different_inode(host.user_namespace.inode)
        }
        InspectionProfile::MountNamespace => {
            host.mount_namespace.inode = different_inode(host.mount_namespace.inode)
        }
        InspectionProfile::Digest => digest = different_hex(&digest),
        InspectionProfile::Epoch => epoch = 8,
        InspectionProfile::Issuer => issuer = 75,
    }
    let key = ed25519_dalek::SigningKey::from_bytes(&[issuer; 32]).verifying_key();
    (host, WorkerPolicyTrust::new(key, &digest, epoch).unwrap())
}
pub(super) fn expected_label(stage: &str, profile: InspectionProfile) -> &'static str {
    match stage {
        "RecordCreated" | "RecordSynced" | "MarkerCreated" => "fresh-refused-authority-invalid",
        "BeforeRefusal" | "DurableCommit" | "AfterReturn" if profile != InspectionProfile::Same => {
            "fresh-refused-authority-binding"
        }
        "BeforeRefusal" => "fresh-refused-authority-active",
        "DurableCommit" | "AfterReturn" => "fresh-refused-authority-refused",
        _ => panic!("unknown context-replay checkpoint"),
    }
}

#[test]
fn diagnostic_context_replay_seeded_inputs_change_one_component_only() {
    let mut seed = 0x074cu64;
    for _ in 0..32 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let host = AssertedWorkerHost {
            machine_id: format!("{seed:032x}"),
            boot_id: format!("11111111-2222-3333-4444-{:012x}", seed & 0xffffffffffff),
            user_namespace: NamespaceIdentity {
                device: 1,
                inode: seed.max(1),
            },
            mount_namespace: NamespaceIdentity {
                device: 2,
                inode: u64::MAX,
            },
        };
        let pin = format!("{seed:064x}");
        let (_, original_trust) = inputs(&host, &pin, InspectionProfile::Same);
        for profile in PROFILES {
            let (changed, trust) = inputs(&host, &pin, profile);
            assert!(crate::diagnostic_worker_policy::valid_context(&changed));
            let host_changes = [
                changed.machine_id != host.machine_id,
                changed.boot_id != host.boot_id,
                changed.user_namespace != host.user_namespace,
                changed.mount_namespace != host.mount_namespace,
            ]
            .into_iter()
            .filter(|changed| *changed)
            .count();
            assert_eq!(
                host_changes + usize::from(trust != original_trust),
                usize::from(profile != InspectionProfile::Same)
            );
        }
    }
    println!("context-replay seed=0x074c same=32 changed=224 component_changes=exactly-one");
}

#[test]
fn diagnostic_context_replay_partial_tail_never_claims_binding_check() {
    for profile in PROFILES {
        for stage in ["RecordCreated", "RecordSynced", "MarkerCreated"] {
            assert_eq!(
                expected_label(stage, profile),
                "fresh-refused-authority-invalid"
            );
        }
        for stage in ["BeforeRefusal", "DurableCommit", "AfterReturn"] {
            assert_eq!(
                expected_label(stage, profile),
                if profile != InspectionProfile::Same {
                    "fresh-refused-authority-binding"
                } else if stage == "BeforeRefusal" {
                    "fresh-refused-authority-active"
                } else {
                    "fresh-refused-authority-refused"
                }
            );
        }
    }
    assert!(
        std::panic::catch_unwind(|| expected_label("unknown", InspectionProfile::Same)).is_err()
    );
}
