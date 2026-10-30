// No automatic exit dump for intentionally killed canaries; the worker publishes
// complete buffer checkpoints through the documented LLVM manual runtime API.
#[cfg(feature = "imap-075-coverage")]
#[used]
#[unsafe(no_mangle)]
static __llvm_profile_runtime: std::ffi::c_int = 0;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = xtask::imap_soak_worker_075::run(std::env::args().skip(1).collect());
    xtask::imap_coverage_075::flush_profile()?;
    result
}
