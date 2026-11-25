//! Conditional atomic updates compatible with the workspace Rust version.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub(crate) trait AtomicCompareUpdate {
    type Value;

    fn compare_update(
        &self,
        success: Ordering,
        failure: Ordering,
        update: impl FnMut(Self::Value) -> Option<Self::Value>,
    ) -> Result<Self::Value, Self::Value>;
}

macro_rules! impl_atomic_update {
    ($atomic:ty, $value:ty) => {
        impl AtomicCompareUpdate for $atomic {
            type Value = $value;

            #[inline]
            fn compare_update(
                &self,
                success: Ordering,
                failure: Ordering,
                mut update: impl FnMut(Self::Value) -> Option<Self::Value>,
            ) -> Result<Self::Value, Self::Value> {
                let mut current = self.load(failure);
                while let Some(next) = update(current) {
                    match self.compare_exchange_weak(current, next, success, failure) {
                        Ok(previous) => return Ok(previous),
                        Err(observed) => current = observed,
                    }
                }
                Err(current)
            }
        }
    };
}

impl_atomic_update!(AtomicU64, u64);
impl_atomic_update!(AtomicUsize, usize);
