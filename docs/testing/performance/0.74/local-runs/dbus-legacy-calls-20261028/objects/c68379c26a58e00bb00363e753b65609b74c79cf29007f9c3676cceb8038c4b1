//! The process table, as the tests of what gets collected read it.
//!
//! A process that has exited and is not waited for stays in the table, as a zombie, until
//! whatever started it collects it. A test that needs to know whether a child was collected looks
//! for its entry under `/proc`, which is why the tests that do are Linux's alone.

/// Whether the process table has an entry for `pid`, a zombie's included.
pub(super) fn is_in_the_process_table(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Waits until the process table has no entry for `pid`.
///
/// Whatever collects a process does it in its own time, so a test that expects the entry to go
/// looks until it has.
pub(super) fn wait_until_collected(pid: u32) {
    while is_in_the_process_table(pid) {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
