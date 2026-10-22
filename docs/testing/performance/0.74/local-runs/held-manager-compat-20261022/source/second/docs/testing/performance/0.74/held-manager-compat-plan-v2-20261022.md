# Corrected local manager reply decoding before mutation

Status: preregistered correction after v1's first read-only refusal, before any
StartTransientUnit was submitted. The original [v1 plan](held-manager-compat-plan-20261022.md)
and source aabcb360 remain unchanged in history. Its first receipt contains only
GetNameOwner, exit 0, with data [":1.2"]. The tool incorrectly expected a scalar.
No unit, job, descriptor transfer or original owner was created in that attempt.

V2 changes only the scalar method-reply decoder to require one argument in
busctl call's top-level data array; the scalar itself retains exact type checks.
The property variant already uses the one-argument envelope. Synthetic controls
use the actual envelope, and a regression replays the captured first response
while rejecting scalar, empty, multiple and wrong-type alternatives.

After committing this correction and passing all ten offline controls on both
platforms, start a distinct fresh v2 compatibility attempt once. Preserve the
failed first output and source. The original unique-manager PID/UID checks,
version-255 restriction, no-preexisting-unit rule, one possible mutation,
zero rights, exact rejection, post-unit absence, closing manager/context checks
and 250 ms / two-second / ten-second test bounds remain unchanged. Existing
runtime predicates, 300/60/10 origins/deadlines and qualification are untouched.

No failed StartTransientUnit is retried: v1 sent none. Any v2 mutation refusal,
timeout or uncertainty is retained once, and this lane stops. This correction
does not admit original delivery, descriptor mapping or writers cessation.
