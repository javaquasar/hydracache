# Owned first Starting transition intent

Baseline is `444ab967da19907261ade8c2fbd1d33e821a3869`. This P1 slice
retains the first Reserved to Starting departure under the existing original
authority and common fixed fence. It is not a worker start or completed cell.
The [intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md)
continues to govern later lifecycle, trusted inputs and server admission.

## Preregistered ownership and storage protocol

Consume the acknowledged `FixedConsumedInitialIntentRead` by value. Require
the owned original early attempt, initial Reserved state and sealed cell-one
record/commit pair. Validate the full initial state against original intent pins
and the exact adjacent revision-two Starting candidate using the existing graph.
No completion count, original identity/origin, cgroup inode or stop reason can
be introduced. Time remains asserted; this cannot authorize execution.

Exclusively create `diagnostic-starting-intent-074.json` in the same fixed leaf.
Use canonical LF-terminated `diagnostic-starting-intent-074-v1`, bounded by
4096 bytes, with previous full encoded-state hash, exact cell-one record hash,
original signed-envelope hash and the full next state. These links are not new
controller signatures. The immutable initial active state remains the anchor;
this separate departure record is not published as a live executing state.

Hold the original file descriptor, device/inode, owner/group, mode, link count,
raw prefix and metadata through create, partial/full write, sealing to 0400,
file/directory sync and final acknowledgement. Revalidate original attempt,
state, cell-one objects, ancestry, fence and original authority/intent at every
boundary. Permit the new filename only while its actual created document is
owned; legacy/unowned namespaces still reject it. Never unlock, reacquire,
adopt, overwrite, unlink, repair or recover an acknowledgement from bytes.

Before mutation, an unbound invalid candidate retains the healthy supplied
guard/fence in the failure. A bound failure keeps the original borrow, lock,
first error, retained context and refusal uncertainty until drop. Success returns
an opaque non-cloneable Starting-intent read with revalidation only; no prior
Reserved/consumed guard, path, FD, state export or start conversion is exposed.
No fresh reader can resume after any partial or unacknowledged departure.

## Preregistered proof

Capture baseline publication/consumption/transition tests and a missing-entry
red test. Cover healthy ownership conservation, invalid and skipped transitions,
legacy input without the original attempt, every logical interruption, partial
bytes, failed acknowledgement, late/foreign targets, original-object replacement,
byte/mode/link drift, original authority/intent expiry and unsafe refusal storage.
Include independent concurrency and seeded candidate mutations.

Add one separately registered ignored/manual nonroot Linux SIGKILL proof:
all eight write/sync/ack stages and death after successful return, plus one
uninterrupted control. Use existing seed `0x074d`, three same-source repeats,
two fresh read-only processes per case, original lock release and owned helper
termination. BeforeCreate already has retained attempt/state/cell-one context,
so fresh reservation must refuse. Rerun all prior process phases because shared
namespace checks and the private crash harness change. This is process loss,
not host reboot or physical power-loss durability.

Run scoped regression, current/MSRV checks, warnings-denied lint/Rustdoc,
format and docs/book checks. Register compatibility, preserve raw negative and
positive evidence, and use a read-only exact-source verifier.

## Closed boundaries

The new record intentionally makes old empty-fence/fixed-state readers refuse
retained context. There is no migration, downgrade cleanup, current-state
overwrite, Starting to Running conversion, cell completion, next-cell intent,
trusted peer/clock/production issuer enrollment, executable/output/watchdog
route, installed-host operation, performance claim, qualification or shipping.
P1 remains open after this first owned departure; P2 through P5 are unchanged.
