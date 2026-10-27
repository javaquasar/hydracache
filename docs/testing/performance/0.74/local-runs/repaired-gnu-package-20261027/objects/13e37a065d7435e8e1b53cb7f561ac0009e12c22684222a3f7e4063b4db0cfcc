//! Exact test-source bridge only; no observation or live execution authority.
use crate::current_security as security;
pub const NAME: &str = "diagnostic_manager_channel.rs";
pub const PATH: &str = "tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs";
pub fn known(name: &str) -> bool {
    matches!(name, NAME | PATH)
}
pub fn project(name: &str, raw: String) -> String {
    if known(name) {
        security::project(PATH, raw)
    } else {
        raw
    }
}
