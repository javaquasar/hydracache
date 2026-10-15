//! Read-only original-controller checks under an owned common lock, not nonce/start admission.
use super::{OidcControllerInput, OidcError};
use crate::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::{
    EmptyReservationFenceError, OriginalEmptyReservationFence,
};
use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum ControllerFenceError {
    #[error("original controller/common-fence preflight permanently refused")]
    Refused,
    #[error("original controller chain refused: {0}")]
    Controller(#[from] OidcError),
    #[error("original common empty fence refused: {0}")]
    Fence(#[from] EmptyReservationFenceError),
}

/// Retains only the original refused fence until drop. No retry or extraction.
pub struct ControllerFenceFailure<'storage> {
    error: ControllerFenceError,
    fence: Box<OriginalEmptyReservationFence<'storage>>,
}
impl ControllerFenceFailure<'_> {
    pub fn error(&self) -> &ControllerFenceError {
        &self.error
    }
    pub fn is_refused(&self) -> bool {
        self.fence.is_refused()
    }
}
impl fmt::Debug for ControllerFenceFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl fmt::Display for ControllerFenceFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl std::error::Error for ControllerFenceFailure<'_> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Fence,
    Controller,
}
const BRACKET: [Step; 5] = [
    Step::Fence,
    Step::Controller,
    Step::Fence,
    Step::Controller,
    Step::Fence,
];

/// Exclusively borrows the original controller and owns its already held fence.
/// No publication, nonce consumption, root enrollment or execution authority.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn copy(g: FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { let _ = g.clone(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn decode<T: serde::de::DeserializeOwned>() {}
/// decode::<FencedControllerPreflight<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>>();
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn send(g: FencedControllerPreflight<'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static, 'static>) { std::thread::spawn(move || drop(g)); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn extract(g: FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { let _ = g.original; }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn start(mut g: FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.start_once(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::OidcControllerInput;
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::reservation_fence::OriginalEmptyReservationFence;
/// fn refresh(g: &mut OidcControllerInput<'_, '_, '_, '_, '_, '_, '_, '_>, f: OriginalEmptyReservationFence<'_>) {
///     let mut bound = g.bind_common_empty_fence(f).unwrap();
///     g.revalidate().unwrap();
///     bound.revalidate().unwrap();
/// }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// use std::os::fd::AsRawFd;
/// fn export(g: FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.as_raw_fd(); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// fn replace(mut g: FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { g.revalidate(b"replacement"); }
/// ```
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::FencedControllerPreflight;
/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::FixtureAuthorityIntentRead;
/// fn promote<'a>(g: FixtureAuthorityIntentRead<'a, 'a, 'a, 'a, 'a, 'a, 'a>) -> FencedControllerPreflight<'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a, 'a> { g.into() }
/// ```
pub struct FencedControllerPreflight<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    original: &'join mut OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    fence: OriginalEmptyReservationFence<'storage>,
}
impl fmt::Debug for FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FencedControllerPreflight")
            .field("refused", &self.is_refused())
            .finish_non_exhaustive()
    }
}
impl FencedControllerPreflight<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn is_refused(&self) -> bool {
        self.original.is_refused() || self.fence.is_refused()
    }
    fn refuse(&mut self) {
        self.fence.latch_refusal();
        self.original.trust.refused = true;
        self.original.original.original.latch_refusal();
    }
    pub fn revalidate(&mut self) -> Result<(), ControllerFenceError> {
        self.revalidate_with(|_| {})
    }
    // Private logical fault boundary only; public callers supply no callbacks.
    pub(super) fn revalidate_with(
        &mut self,
        mut before: impl FnMut(Step),
    ) -> Result<(), ControllerFenceError> {
        if self.is_refused() {
            self.refuse();
            return Err(ControllerFenceError::Refused);
        }
        let result = (|| {
            for step in BRACKET {
                before(step);
                match step {
                    Step::Fence => self.fence.revalidate()?,
                    Step::Controller => self.original.revalidate()?,
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.refuse();
        }
        result
    }
}
impl<'o, 't, 'r, 'c, 'b, 's, 'a, 'p> OidcControllerInput<'o, 't, 'r, 'c, 'b, 's, 'a, 'p> {
    pub fn bind_common_empty_fence<'join, 'storage>(
        &'join mut self,
        fence: OriginalEmptyReservationFence<'storage>,
    ) -> Result<
        FencedControllerPreflight<'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        ControllerFenceFailure<'storage>,
    > {
        let mut read = FencedControllerPreflight {
            original: self,
            fence,
        };
        match read.revalidate() {
            Ok(()) => Ok(read),
            Err(error) => Err(ControllerFenceFailure {
                error,
                fence: Box::new(read.fence),
            }),
        }
    }
}
