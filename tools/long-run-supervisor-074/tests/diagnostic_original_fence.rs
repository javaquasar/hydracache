#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::{
    open_fixed_intent_storage, reserved_state::original_fence::bind_original_host_fence,
};

#[test]
fn original_fence_public_entry_has_no_caller_selected_origin() {
    let _entry = bind_original_host_fence;
    // Nonroot local tests cannot manufacture a fixed-root enrollment.
    assert_ne!(unsafe { libc::geteuid() }, 0);
    assert!(open_fixed_intent_storage().is_err());
}

#[test]
fn empty_reservation_fence_public_entry_has_no_caller_selected_origin() {
    use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::bind_original_empty_reservation_fence;
    let _entry = bind_original_empty_reservation_fence;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    assert!(open_fixed_intent_storage().is_err());
}

#[test]
fn empty_fenced_intent_public_entries_require_original_guard_borrows() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::empty_fence::{bind_fixed_empty_fenced_intent, bind_fixture_empty_fenced_intent};
    let _fixed = bind_fixed_empty_fenced_intent;
    let _fixture = bind_fixture_empty_fenced_intent;
    assert_ne!(unsafe { libc::geteuid() }, 0);
    assert!(open_fixed_intent_storage().is_err());
}
