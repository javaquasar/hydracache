# Scoped attribution of original kernel refusals

The [local contract](diagnostic-kernel-refusal-attribution-contract.toml) adds
test-only first-refusal capture to the original controller fixture. It does not
change admission or explain the earlier uninstrumented Kernel(Invalid) by itself.
The [preregistration](diagnostic-kernel-refusal-attribution-plan-20261016.md)
starts from `914aa749` and preserves that packet as historical evidence.

## What a trace means

Thirty-two Invalid construction sites in kernel admission and origin now use
two private macros. The ordinary build still constructs KernelError::Invalid
and keeps its original three Option::ok_or expressions. Only cfg(test) enables
recording and makes those three constructors lazy, preventing healthy Option
checks from emitting a false error event. Ancillary parsing is unchanged.

The capture module exists only under cfg(test). A stack-scoped thread-local
frame retains the first static site, file/line, fixture phase and six unsigned
clock facts. Changing phase or encountering another error cannot overwrite
that event. Nested scopes save and restore the original frame, including on
unwind; eight simultaneously active test threads retain independent frames.
Panic reporting is best effort and preserves the original panic payload.

Phases distinguish packet construction, worker binding, controller admission,
reviewed build binding, OIDC binding, the test callback and postconditions.
An event describes the first Invalid in its scope, not necessarily the cause of
a later assertion failure. Expected negative tests can legitimately produce
events; positive scopes produce none. Outside a scope, recording is disabled.

For clock-order and packet-hold sites the six facts are, in order: before and
after boottime nanoseconds, before and after realtime nanoseconds, whether boot
identity matches, and the unchanged hold limit in nanoseconds. Other sites
retain zeros. No extra clock or proc observation is taken to build an event.
No packet, bearer token, raw credentials, public configuration body or key is
retained. The captured numeric facts are diagnostics, never epoch authority.

## Guard preservation

The independent static test compares both non-test source prefixes against the
exact baseline after removing only the approved macro/module header and
expanding the two macros to their original constructors. It ignores only
non-semantic question-mark joint spacing introduced by line wrapping. It does
not erase arbitrary blocks, reorder observations or normalize predicates.
Deadline and filesystem-guard canaries must still differ from baseline.

The later [original collector preflight](diagnostic-original-collector-design.md)
adds one private kernel module. Its exact two-line registration is now excluded
before comparing the old guards; no function or predicate is skipped. The added
module has its own contract gate. Historical instrumentation-only source claims
remain scoped to the original pinned packet, not this larger module graph.

An initial comparison correctly rejected a rustfmt-introduced closure block.
Shortening that site's static label restored the original expression shape;
the comparison was not generalized to discard blocks. A separate development
assertion initially expected 33 sites; the actual matching constructor inventory
is 32. The first strict Clippy run also rejected globally lazy constructors in
the ordinary build. The explicit cfg(test)/non-test Option macro preserves the
original normal expression instead of suppressing the lint.

This is restricted source equivalence, not binary reproducibility or a measured
allocation/throughput result. Error schema, budgets, hold boundary, refusal
latches, authority, workload and qualification inputs remain unchanged. There
is no new public producer, runtime journal, retry or fallback.

## Reproduction and interpretation

Deterministic tests classify forward/reversed clocks, the exclusive 60-second
hold boundary, timespec overflow, namespace type/object and proc-origin drift,
and strict credential-projection failures. They verify lazy healthy checks,
redaction, first-event retention, nesting, unwind and thread isolation. The
classification order uses seed `0x0740c096`.

Bounded repeated original-input runs may expose a new real first-refusal event.
Such an event identifies only that occurrence. A passing repetition, an injected
model refusal or a nearby timesync journal cannot establish the historical
failure's cause. Preserve every failed run and keep original-start, all-writer
cessation, production trust, server operations and qualification closed.

The bounded local run completed three original-input repetitions on Rust 1.94
and one on Rust 1.88: each passed 69 tests with the existing manual case ignored.
None produced an unexpected first-refusal event. The kernel group passed 25
tests on each compiler, including the classifier canaries. This makes the
diagnostic capability available; it does not close historical attribution.
The [retained packet](local-runs/kernel-refusal-attribution-20261016/README.md)
distinguishes those observations from injected negatives and development errors.
