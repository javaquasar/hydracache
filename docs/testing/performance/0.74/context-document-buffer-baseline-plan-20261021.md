# Pure baseline for local context document scratch

Freeze the unchanged worker-context `Document::read_with` before a separate
implementation candidate. This package measures only Rust allocation requests
for its zeroed `vec![0; length + 1]`, using lengths 0, 1, 4, 32, 33, 36, 37, 38
and 63, 256 constructions each. The 33/37 production lengths and length-four
internal fixture are distinguished from additional shape controls. Do not time
Probe, read proc/sys files, construct a live owner or extrapolate old span counts.

Retain a pure frozen body model with only the two metadata operands replaced by
synthetic Result-valued observations. Whole-body token comparison must preserve
every other operation, short circuit, read slice and error propagation. Check
zeroed exact callback slices, seeded bytes, short/zero/overrun read counts,
content/newline/expected-length errors, EINTR/EIO/EBADF, metadata failures before
and after reading, and pre-metadata refusal before allocation arithmetic.
These are model controls, not proof of real kernel metadata races or EOF/fence.

The only old supervisor source edit may be an opt-in allocation-test module
hook. Exact source reversal must restore its entire old file; worker context
and every other old supervisor file must remain unchanged at `8717ea4f`.
Do not implement the stack replacement, narrow the private length argument,
modify validator counts/order/deadlines, cache observations or alter a refusal.

Run selected pure unit controls on GNU 1.94 and MSRV 1.88, Windows source
canaries, affected all-target checks, strict all-feature lint, formatting and
local structural gates. No ignored/manual test, original-owner/live profile,
clock/service change, installation, supervisor, server or product workload.
Verify all five earlier evidence packets by raw seals without editing their
frozen reviewers to accept this test-only successor. Keep first Causal(Time)
refusals, original 300/60/10-second windows and qualification inputs unchanged.
