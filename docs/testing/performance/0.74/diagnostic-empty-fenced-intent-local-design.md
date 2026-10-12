# Signed intent under the original empty reservation fence

A future initial Reserved publisher needs authority and emptiness checked under
one retained common lock. This package only joins the existing original signed
authority and intent guard to the existing original empty reservation fence.
Baseline is `90527b85aec704dfcfe3b41f2b4142022b7f976a`. Publication,
host ownership, installer, authentication and worker execution remain closed.

Distinct fixed and fixture wrappers exclusively borrow both original guards.
The complete asserted Reserved snapshot must equal the original signed-intent
pins before I/O. A wrong unbound snapshot does not revoke healthy inputs.
There is no persisted snapshot in the empty leaf: this match is not evidence
of a durable reservation. Neither original guard is reopened or copied.

Observe fence, intent, fence, intent, fence under the caller's already held
original lock. Each fence observation retains fresh independent leaf scans
and original ancestry, lock and legacy-context checks. Each intent observation
retains its authority, signed projection, authority bracket. Stop on the first
error; these sequential observations do not guarantee continuous confinement.

A bound failure memory-latches both original chains through wrapper drop and
restoration. Fence failure attempts Policy refusal only through the original
authority journal's independently exact-safe bridge. Unsafe or advanced authority
must not be appended. Preserve the original failure and typed acknowledgement
uncertainty. Intent failure uses its existing refusal bridge once, without a
second append. An already refused input cross-latches the other before I/O.
Successful join drop must leave the caller's original empty fence locked.

Tests cover every bracket error position, repeated healthy reads and drop,
unbound full-snapshot mismatch, expiry/signature/state/peer/worker drift, early
and late fence conflicts, unsafe/advanced and partially published authority
refusal, sticky refusal, fixed private dispatch, thread-only wrappers and
independent concurrent chains. Retain baseline and absent-public-API red first,
then repeated focused tests, broad Linux and Windows regression, compile-fail
API checks, current/MSRV lint/format/docs and actual qualification/ship refusal.
Private nonroot models use synthetic keys and temporary locks, not enrollment.

The next separate contract must consume or transition this same original
empty fence into no-overwrite state publication while retaining its original
lock and exact published object. Writing a file and then revalidating an empty
guard necessarily refuses; dropping and reacquiring the lock is not a safe
transition. Partial/crash uncertainty, authenticated peer/clock, coordinated
trusted ownership and binary rollout still precede any host pilot. Frozen
qualification inputs and their inherited main-plan digest refusal stay unchanged.
