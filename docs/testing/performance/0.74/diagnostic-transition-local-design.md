# Diagnostic state transition contract

This first P1 slice defines adjacent state transitions for the four fixed cells.
The starting source is `cd01ddbee8f0ba405869a639c78bc16eb5852d37`.
It strengthens the local model publisher, not the original fixed publisher:
cells two through four still cannot be consumed or executed through fixed guards.
The governing sequence is the
[intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md).

## Preregistered transition rules

Both states must satisfy the existing schema-1 state validator. Lease, boot,
binary, build provenance and reservation origin remain identical. An adjacent
publication advances revision by exactly one, without overflow; observation and
controller time cannot go backwards. A controller refresh must equal the new
observation time and precede the old controller-loss boundary. These are checks
of asserted values, not authentication of a production clock or peer.

| Previous stage | Next stage | Required change |
| --- | --- | --- |
| Reserved | Reserved | Observation or heartbeat only; same cell count |
| Reserved | Starting | Same count; fresh start equals observation time; unexpired controller and total budget |
| Reserved | Terminal | Same count; cancellation, controller loss or total deadline; no process was started |
| Starting | Starting or Running | Same count and original start; no stop reason |
| Starting or Running | Stopping | Same count and original start; retain a non-completion stop reason |
| Running | Running | Same count/start; an absent cgroup inode may bind once |
| Running | Reserved | Exactly one completed cell, fewer than four; cleanup confirmed; start and inode cleared |
| Running | Terminal | Exactly the fourth completed cell; Completed reason and cleanup confirmed; original start/inode retained |
| Stopping | Stopping or Terminal | Same count/start/reason; terminal requires cleanup confirmed |
| Terminal | Any stage | Refuse, including identical replay |

Once an inode is bound it cannot change or disappear, except at successful
Running-to-Reserved completion. A stop reason cannot change or disappear.
No cell may be skipped, decremented, repeated as a new start, or completed from
Starting/Stopping. Validation is side-effect free. Model publication reads the
previous state under its existing transaction fence and validates the adjacent
candidate before writing. No new lock, filename, durable schema, CLI, execution
permission or recovery conversion is introduced.

## Test and evidence sequence

Retain an unmodified local-model baseline, then compile the new private-validator
tests before implementation to retain the missing-function red result. Cover
the entire five-by-five stage graph, all four cells, terminal/revision replay,
identity/time/count/reason/start/inode drift, expiry and revision overflow.
Use deterministic seed `0x074e` for generated legal histories and invalid
mutations. Run three focused repeats, existing lifecycle/IPC regressions and the
existing campaign/diagnostic concurrency test. Run package format, all-target
check and Clippy with warnings denied on Windows and Linux, plus documentation
checks and the MSRV check. Preserve exact commands, source identity and raw logs
in a separate local packet. A full release gate remains a milestone requirement.

## Boundaries still open

State data and successful validation do not prove durable intent consumption,
signed history, authenticated outcomes, original writer cessation or empty
cgroup. The local backend remains a test seam. Fixed progression still needs an
owned protocol preserving the original immutable state/intent bindings and
fence; reopening or rebinding them is not an implementation strategy.

The pre-create P0 crash leaves no durable reservation or consumed nonce. Early
attempt admission, cells two through four, previous-record hash chaining,
partial/uncertain history, and reboot handling remain separate P1 work. New write
boundaries will require their own compatibility registration and actual process
loss matrix. P2 through P5, product candidate admission and qualification stay
closed; frozen manifest inputs and thresholds are unchanged.
