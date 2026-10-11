# Local durable signed intent retention

This fixture-only slice tests durable consumption before any future execution.
It does not add a backend, start permission, issuer enrollment or production path.
Baseline is `c690a08b3a52a6545e0d1ea27ef6188e8e6aed16`.
The existing signed projection and schema-1 Reserved state remain unchanged.

The API requires the original checked intent, exact signed bytes and external
check assertions. It first revalidates that guard, then acquires the existing
cooperative host fence. Writes are limited to a nonroot-owned 0700 campaigns
directory under a direct /tmp/hydracache-intent-* temporary directory; production
roots, symlink ancestry and missing/replaced lock objects refuse. Exact private
persisted state and the existing pending/conflict/IPC checks bracket publication.
All failures latch the checked intent, without repairing or revoking an unrelated
worker policy. An unsafe journal is never appended merely to publish a refusal.

Four fixed cell-index record/marker pairs form a contiguous history. A version-1
record contains full Reserved state, the exact signed envelope as a UTF-8 string,
asserted peer UID/groups and the previous record digest. Canonical sorted-key JSON
plus LF is bounded at 16384 bytes; a marker is lowercase record SHA-256 plus LF,
exactly 65 bytes. The directory inventory is bounded at 64 total names; any unknown
intent-prefixed name, missing pair, gap or over-four-cell history refuses.

History signatures are rechecked under the original controller configuration,
worker policy and asserted peer/boot. Historical validity-window grammar is
checked at its signed issue instant solely to authenticate consumed records,
never to renew launch eligibility. Lease/build identity and controller IDs stay
exact; revision and asserted monotonic observations increase/nondecrease, cells
are contiguous, and request IDs/nonces cannot repeat. Current history length must
equal the asserted completed-cell count. Already consumed cells refuse even with
a new valid signature. Advancing completed cells is an external state assertion,
not proof that a backend executed or cleaned up.

Publication uses exclusive descriptor-relative create, directory fsync, bounded
record write, readonly 0400 sealing/file fsync, then an exclusive marker with the
same sealing/fsync and final directory fsync. Original/named records, state,
ledger, ancestry and lock are checked again. Logical faults after record creation,
record sync, marker creation and final durable commit retain every file and latch
the guard with no retry. A complete late-unacknowledged pair remains consumed;
reopening it is inspection only, never a second start. There is no overwrite,
rename, truncate, remove, eviction, restart or recovery adoption.

Tests cover four-cell close/reopen, duplicate/new-signature replay, same-box
concurrent retention, boot/controller/worker changes, expired signatures,
canonical/schema/hash-chain/metadata/budget failures, unsafe links/special files,
state/pending/IPC conflicts, replaced root/lock/state/records, all four logical
publication faults, and guard refusal after restoration/drop. Synthetic keys and
owned temporary fixtures are the only positives; no physical power-loss proof,
root/owner-tamper resistance or whole-host snapshot rollback is claimed.

Remaining production work is a fixed root-owned backend and original-authority
join, authenticated kernel peer/clock and original material/output/runtime
composition, fenced Starting/watchdog/cleanup and controlled host enrollment.
An absent journal cannot prove there was no prior execution before this protocol;
no legacy start is adopted. Frozen qualification inputs and admission remain
unchanged. No performance workload or numerical claim accompanies this slice.
