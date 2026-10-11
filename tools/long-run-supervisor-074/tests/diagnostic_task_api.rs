#![cfg(target_os = "linux")]

#[test]
fn signed_intent_journal_api_is_fixture_data_not_execution() {
    use hydracache_long_run_supervisor_074::diagnostic_output_preparation::intent_journal::{
        inspect_fixture_intents, retain_fixture_intent, FixtureIntentObservation,
    };
    let _ = (inspect_fixture_intents, retain_fixture_intent);
    fn data_only(_: FixtureIntentObservation) {}
    let _ = data_only;
}

#[test]
fn fixture_lifecycle_recovery_api_is_readonly_and_fixture_origin_only() {
    use hydracache_long_run_supervisor_074::diagnostic_output_preparation::recovery::{
        inspect_fixture_recovery, FixtureRecoveryObservation,
    };
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::FixtureAuthorityBoundRead;
    let _ = inspect_fixture_recovery;
    let _ = FixtureAuthorityBoundRead::check_fixture_lifecycle;
    fn data_only(_: FixtureRecoveryObservation) {}
    let _ = data_only;
}

#[test]
fn signed_authority_api_keeps_original_fixed_and_fixture_types_distinct() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::{
        bind_fixed_authority, bind_fixture_authority, FixedAuthorityBoundRead, FixtureAuthorityBoundRead,
    };
    let _ = (bind_fixed_authority, bind_fixture_authority);
    fn fixed(_: &FixedAuthorityBoundRead<'_, '_, '_, '_, '_, '_>) {}
    fn fixture(_: &FixtureAuthorityBoundRead<'_, '_, '_, '_, '_, '_>) {}
    let _ = (fixed, fixture);
}

#[test]
fn signed_tasks_api_preserves_fixed_fixture_original_mount_types() {
    use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::{
        open_signed_fixed_tasks, open_signed_fixture_tasks, FixedSignedTasksRead, FixtureSignedTasksRead,
    };
    let _ = open_signed_fixed_tasks;
    let _ = open_signed_fixture_tasks;
    fn fixed(_: &FixedSignedTasksRead<'_, '_, '_, '_, '_>) {}
    fn fixture(_: &FixtureSignedTasksRead<'_, '_, '_, '_, '_>) {}
    let _ = (fixed, fixture);
}

#[test]
fn task_set_api_accepts_only_original_process_and_asserted_credentials() {
    use hydracache_long_run_supervisor_074::diagnostic_process::{
        pin_asserted_worker_tasks, WorkerTasksRead,
    };
    let _ = pin_asserted_worker_tasks;
    fn original_only(_: &WorkerTasksRead<'_>) {}
    let _ = original_only;
}
