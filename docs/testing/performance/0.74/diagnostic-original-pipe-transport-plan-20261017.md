# Original collector pipe message preparation

Baseline is `fa680f3d4220bb0e49bd730c990ff07b9edfe8fa`. The original
material owner retains signed requirements, bundle, Starting, pipes, sinks and
fence, but cannot represent Unix FD properties in the legacy systemd transport.
Prepare a distinct opaque, consuming message owner without connecting to a bus
or sending a method call. Do not modify the legacy UnitProperty enum or transport.

## Contract

The no-argument entry consumes original material. Fresh full original live
rounds bracket material and message inspection. Reconstruct only the fixed
collector worker settings from the original Reserved state, compare every
property and its typed value to the retained signed projection, and use the
same two original write ends for StandardOutputFileDescriptor and
StandardErrorFileDescriptor. Encode them as D-Bus Unix FD values, not numeric
properties. The message owns two known duplicate writers; it drops before the
original material/fence. No evidence sink FD enters the message.

Construction failure and later validation failure latch the original chain;
failure retains the owner. There is no extraction, cloning, deserialization,
connection, send, start, retry or completion API. A cfg(test) fixture path can
only preserve fixture origin. Do not reinterpret a legacy file-output signature
as collector authority. Preserve all original checks, schedules and deadlines.

The signed RuntimeMaxUSec is a requirement ceiling, not a fresh execution
budget. This unsent message cannot authorize execution later: a future start
must derive the residual absolute cell/total deadline, arm its independent
watchdog before send, bind manager/process/cgroup identities, and account for
all creator/manager writer copies. No runtime-origin refresh is allowed.

## Verification

Retain a missing-API compilation failure before implementation. Test all typed
settings against signed requirements; binary bytes, stdout/stderr separation,
message FD identities/flags, no sink descriptors, EOF only after the final
message writer copy closes, and seeded setting mutations. Decode actual zbus
message bytes and its out-of-band FD table without any bus connection.

Run current/MSRV units, API/compile-fail checks, affected all-target check and
strict Clippy. Run three complete original fixture chains per compiler, plus
independent message/bundle/output/trust/origin failures with exact errors,
sticky refusal, unchanged intent files and fence ownership. Use only the
already authorized nonroot isolated synthetic server lane; no installation,
supervisor change or product execution. Keep raw logs and exact source/binary
hashes. This adds no durable write boundary and cannot replace the prior
Starting/output SIGKILL proof or the future execution/writer crash matrix.

Watchdog implementation, actual transport send and original writer handoff are
the next execution slice; installed enrollment and integrated host admission
remain open. No qualification manifest or numerical threshold changes here.

## Completed message preparation

Implementation `e2e92fd9` passes three complete original fixture chains and nine
exact refusals per compiler, including a pre-refused owner and original real
progress-age expiry. Construction and revalidation each observe 34
authentications, two fresh full original rounds. The
[packet](local-runs/original-pipe-transport-20261017/README.md) retains raw results,
source/binary hashes, tooling failures and independent unchanged-host postflight.
Only unsent message representation/ownership is closed. Authenticated delivery,
armed watchdog, actual writer handoff and every execution/lifecycle gate remain.
