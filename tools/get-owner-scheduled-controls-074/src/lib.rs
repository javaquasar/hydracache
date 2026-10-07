//! Local instrumentation, never a product or qualification dependency.
//! Reuse the canonical schedule and Target interface by source reference so
//! the historical loadgen global counting allocator is not linked here.
#[path = "../../../crates/hydracache-loadgen/src/histogram.rs"]
pub mod histogram;
#[path = "../../../crates/hydracache-loadgen/src/rate.rs"]
pub mod rate;
#[path = "../../../crates/hydracache-loadgen/src/target.rs"]
pub mod target;

pub mod native;
pub mod scheduled;
