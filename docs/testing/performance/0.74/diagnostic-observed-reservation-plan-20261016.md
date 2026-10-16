# Initial Reserved state from the original receiver clock

Baseline `0389978f1ff5d7eeacb99c497f9ef8cbfd6be08a`. The original collector
preflight authenticates a supplied state but does not authenticate its clock
fields. Before durable collector lifecycle publication, add a separate consuming
owner of the acknowledged ControllerRequestAttempt. Do not combine the legacy
fixed authority/regular-file intent chain with this original controller chain.

The new owner obtains boot identity and CLOCK_BOOTTIME from its retained kernel
packet, bracketed by the existing worker/authority/controller/storage checks.
Mint revision-one, cell-zero Reserved once in memory from the exact request
identity. Freeze all three initial clock fields to that first sample. Later
observations check boot and clock order, without changing that state or renewing
the original packet hold or request window. Returning a cloned state for signing
exports data only; changing the clone must not change the owner.

Bind collector signatures through a distinct borrowed guard accepting bytes
only, using the frozen observed state and existing original signature/request
checks. No conversion from the supplied-state guard, raw timestamp, peer, path,
descriptor or legacy authority is available. Consuming the request prevents a
second reservation owner. Any failure cross-refuses the retained original chain;
later calls do no I/O. Both failure and success keep the original fence until
owner drop. No new journal opener, overwrite, repair, adoption or file is added.

Capture a baseline and missing-method compiler red before implementation. Cover
actual kernel clock brackets, unchanged bytes/inventory/fence, stable repeated
state export, signed mutation refusal (seed 0x0740c099), pre-refusal, late source
and storage drift, independent concurrent owners and compile-fail API boundaries.
Run current/MSRV targeted tests, strict package check/Clippy, format, static gates,
registry/doc checks and closed admission checks. Retain source-pinned local logs.

This is observed in-memory state, not durable Reserved/Starting acknowledgement,
root ancestry/output construction, original-start authority or reboot recovery.
Next publish it under the same original owner/fence through a separately
preregistered additive durable protocol, including its SIGKILL/lost-ack matrix.
No new write boundary exists in this slice. No product, frozen qualification,
server, workflow, expensive workload or push changes. Kernel(Invalid) remains
unexplained. Production enrollment and all P1-P5 execution gates stay open.
