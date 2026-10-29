//! Fresh validated UUID equality without temporary ownership; never a cache.
use super::*;

pub(super) fn matches(root: &File, expected: &str) -> Result<bool, KernelError> {
    boot_apply(root, |raw| view(&raw).map(|value| value == expected))
}
pub(super) fn view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {
    boot_view(raw)
}
