//! Separate numeric-root ancestry; old nonroot storage helpers stay exact.
use super::*;
use crate::host_execution::HOST_EXECUTION_LOCK_NAME;
use reserved_state::original_fence::reservation_fence::{
    bind_original_empty_reservation_fence, OriginalEmptyReservationFence,
};
use std::os::unix::fs::PermissionsExt;
pub(crate) fn with_fence<R>(
    run: impl FnOnce(OriginalEmptyReservationFence<'_>, &Path, &Path) -> R,
) -> R {
    let root = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
    let mut leaf = root.path().to_owned();
    for name in COMPONENTS {
        leaf.push(name);
        std::fs::create_dir(&leaf).unwrap();
        std::fs::set_permissions(&leaf, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let campaigns = root.path().join("var/lib/hydracache-performance/campaigns");
    std::fs::create_dir(&campaigns).unwrap();
    std::fs::set_permissions(&campaigns, std::fs::Permissions::from_mode(0o700)).unwrap();
    let lock = campaigns.join(HOST_EXECUTION_LOCK_NAME);
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(lock)
        .unwrap();
    let mut storage = walk(root.path(), 0, 0).unwrap();
    let fence = bind_original_empty_reservation_fence(&mut storage).unwrap();
    run(fence, &campaigns, &leaf)
}
