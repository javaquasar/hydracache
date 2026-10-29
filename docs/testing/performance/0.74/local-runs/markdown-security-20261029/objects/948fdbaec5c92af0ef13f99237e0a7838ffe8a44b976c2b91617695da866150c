//! Test-only ownership-preserving call adapter; never replaces a validation.
use super::*;
impl<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
    FixedPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>
{
    pub(crate) fn join_held_fixture(
        self,
        roles: PinnedRoles,
        trust: HeldTrustInput,
        documents: [&[u8]; 4],
    ) -> Result<
        JoinedHeldPlacement<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
        HeldJoinFailure<'root, 'join, 'storage, 'o, 't, 'r, 'c, 'b, 's, 'a, 'p>,
    > {
        let [held_policy, worker_policy, receipt, binding] = documents;
        self.join_held_roles(
            roles,
            trust,
            JoinDocuments {
                held_policy,
                worker_policy,
                receipt,
                binding,
            },
        )
    }
}
impl JoinedHeldPlacement<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn fixture_origins(&self) -> ([u64; 4], [u64; 4]) {
        self.placement.held.placement_origins_for_fixture()
    }
    pub(crate) fn fixture_timer_live(&self) -> bool {
        self.placement.held.placement_timer_live_for_fixture()
    }
}
impl HeldJoinFailure<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(crate) fn fixture_timer_live(&self) -> bool {
        self.held.fixture_timer_live()
    }
}
