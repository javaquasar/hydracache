//! Synthetic signing data only; this module is absent from ordinary builds.
use super::*;
use crate::diagnostic_two_phase_launch::{held_placement as wire, LaunchArtifacts, LaunchBody};
impl OwnedRootGuardians<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn placement_for_fixture(
        &self,
        identity: [u64; 2],
    ) -> (wire::PlacementBody, RootBindingClaims) {
        let material = &self
            .owner
            .original
            .original
            .original
            .original
            .original
            .transport
            .material;
        let consent = material
            .outputs
            .pipes
            .starting
            .original
            .original
            .prepared
            .consent_to_sign()
            .clone();
        let artifacts = LaunchArtifacts {
            launcher_sha256: "d".repeat(64),
            watchdog_sha256: "e".repeat(64),
            collector_sha256: "f".repeat(64),
        };
        let launch = LaunchBody {
            schema_version: crate::diagnostic_two_phase_launch::BODY_SCHEMA.into(),
            action: "prepare-held-cell-one".into(),
            requirements_sha256: crate::sha256_hex(
                &crate::diagnostic_two_phase_launch::launch_requirements(&consent, &artifacts)
                    .unwrap(),
            ),
            consent_claims: consent,
            artifacts,
        };
        let root = self.owner.binding(identity).unwrap();
        let packet = PacketClaims {
            kind: Kind::Enroll,
            binding: root,
        }
        .encode()
        .unwrap();
        (
            wire::PlacementBody {
                schema_version: wire::BODY_SCHEMA.into(),
                action: "project-held-cell-one".into(),
                root_enrollment_hex: packet.iter().map(|b| format!("{b:02x}")).collect(),
                requirements_sha256: crate::sha256_hex(
                    &wire::placement_requirements(&launch, &root).unwrap(),
                ),
                launch_claims: launch,
            },
            root,
        )
    }
}
impl OwnedPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn placement_origins_for_fixture(&self) -> ([u64; 4], [u64; 4]) {
        self.held.fixture_origins()
    }

    pub(crate) fn placement_timer_live_for_fixture(&self) -> bool {
        self.held.controls.placement_timer_live_for_fixture()
    }
}
impl PlacementFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn placement_timer_live_for_fixture(&self) -> bool {
        self.held.controls.placement_timer_live_for_fixture()
    }
}
