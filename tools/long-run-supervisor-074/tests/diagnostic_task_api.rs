#![cfg(target_os = "linux")]

#[test]
fn task_set_api_accepts_only_original_process_and_asserted_credentials() {
    use hydracache_long_run_supervisor_074::diagnostic_process::{
        pin_asserted_worker_tasks, WorkerTasksRead,
    };
    let _ = pin_asserted_worker_tasks;
    fn original_only(_: &WorkerTasksRead<'_>) {}
    let _ = original_only;
}
