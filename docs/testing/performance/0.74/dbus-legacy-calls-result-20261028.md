# Whole legacy systemd helper compatibility on the zbus bridge

The standalone source-package consumer now passes **78/78 tests on GNU Rust
1.94 and 1.88**, with check and strict Clippy on each compiler. The original
67 cases remain alongside eleven new functions exercising eight whole legacy
helper bodies. This is a private compatibility gate, not supervisor integration,
authenticated original-start, host/service execution or release/server admission.

## Exact bodies and the connection seam

The whole original systemd_unit.rs is pinned to SHA-256
`4b8f35e341b489f5928206b2df3f09525abb081b937e35cefd5c1ca1bad01cac`.
Its selected type definitions, start_transient_unit, inspect_unit,
inspect_unit_optional, stop_unit_and_wait, reset_failed_unit_and_wait,
inspect_loaded_unit, dbus_properties and validate_unit_name bodies are copied
without edits. Four original Connection::system acquisition sites resolve to a
private test-only provider. Each successful acquisition receives a fresh private
p2p connection. The type imports resolve through the tested blocking bridge and
zbus 6 Variant spelling. No CampaignManifest/policy builder, kernel/live frame
or original-owner function is supplied as a successful stub.

The new private module compiles only under cfg(test). Production root callers,
manifest/lock/deny policy, source-package compiler Rust/features, the 62-node
consumer lock and frozen qualification identity remain unchanged. This does not
compile the full systemd_unit module or the complete supervisor/module closure.
A fresh lexical census pins all 18 direct zbus/wire-use source files and their
references/imports; it is explicitly not an AST, macro or transitive closure proof.

## Actual scripted coverage

Each image runs 24 scenarios under eleven test functions. The 23 fixed scenarios
require 34 provider acquisitions, including a deliberate acquisition error. All
requests check destination, object, interface, member and decoded body. Start
independently checks all six UnitProperty forms, their order, mode fail, command
argv/ignore-failure and empty auxiliary units. Stop requires mode replace; reset
requires the original unit argument. Lazy cache GetNameOwner/GetAll reads remain
in the scripts and count as actual traffic rather than being hidden by a cache.

Bad reply types, manager errors, a real close of the private channel and a
withheld acknowledgement return an error without another send. This lost-ACK
control proves only the legacy helper's lack of automatic retry: it is not a
durable lifecycle/SIGKILL or original-start uncertainty proof. Invalid namespace
and unsupported timeout values refuse before provider access. Inspect wrappers
keep optional absence, complete lazy property reads and type-error refusal.

Stop accepts inactive/dead/zero PID or post-stop absence, polls active/exited
rather than reporting it stopped, and obtains a fresh inspection connection.
Reset rejects non-failed state, wrong substate, nonzero PID and nonempty cgroup
before ResetFailedUnit. It continues polling a still-failed unit and accepts only
the existing absent or empty inactive shape. These are scripted observations,
not proof that real processes or hidden writers have stopped.

A final scenario invokes the original supported one-second stop timeout while
all observations remain active/running. It reaches the unchanged StopTimeout
branch. The deadline is still created once before the loop; both original
one-origin loops and 100-ms sleeps remain byte-exact. Twelve acquisitions are
observed per image in this run, including the initial stop; that scheduling-
dependent count is not an acceptance threshold or timing/performance claim.
All other transcripts agree exactly between compilers.

## First refusals and their cause

The first complete 78-case run passes 68 and fails ten: all 67 predecessor cases
and namespace/policy control pass. The scripted peer closes after a construction
error, leaving BrokenPipe in the caller. Its cleanup originally reduced the peer
JoinError to a boolean. A separate diagnostic input retains the same 68/10 verdict
and prints InvalidName: the fixture used the well-known systemd destination as
reply Sender, which must instead be a unique name. This is a fixture error, not
evidence that a real manager reply or the copied caller failed authentication.

The first attempted repair changed only error replies; normal replies retained
the invalid sender and again produce 68/10. That distinct source/result remains.
The final fixture fixes both reply branches to :1.1 while requests still target
the original well-known destination. A cleanup accumulator also ensures every
peer is joined/closed even if an earlier peer fails; it does not short-circuit.
No caller predicate, assertion, catalog entry, timeout or sleep is relaxed.

An additional inert control initially fails because it expects a multiline
Rustfmt layout; its raw 7/8 outcome remains. Matching the same exact sender
expression independently of whitespace restores all eight controls. Rust
formatting passes. No failed output is overwritten or unchanged native command
retried to turn green. The final actual native catalogs select all 78 identities
with zero failures, ignored or filtered cases on both compilers.

## Ownership, limits and remaining boundaries

The private upstream Channel deliberately bypasses OS authentication. The
fixtures address neither a system/session bus nor real services. Task failures
close their private endpoint; each fixture completes or aborts/joins its own task
before releasing the endpoint. CI owners remain the pinned Windows Job and Linux
subreaper/pidfd/ECHILD helpers. This cleanup is not production all-writer custody.

The original local 1200-second outer/1100-second inner absolute origins, four-GiB
floors, five-second polling/retirement and 250-ms fixture method/receive/join guards
remain. The 250-ms private connection timeout is fixture containment, not a claim
about legacy Connection::system's default. Original live 300/60/10 and package
780/workspace 4800 guards are untouched. No clock/service/installation/server,
product workload, qualification or new performance hypothesis is run.

The [primary packet](local-runs/dbus-legacy-calls-20261028/index.json) has
2,716 logical members / 489 raw objects, all four frozen source contexts and
first complete native outcomes, final compiler/catalog/accounting, direct-use
census, eight controls and formatting. Index SHA-256:
`eb82212e5e93a10017712b4499ad4b288e8e1018801aa0d64b9184a04e3417f6`.
Native executable bytes/build trees are excluded; actual image receipts remain.
Source implementation commit: `fae972a0fbba429401e39472206408ffd276f9a7`.

Next complete the real caller/module closure and source/effect review, including
FixedManagerChannel, its kernel/owner frame and held/original delivery. Registry
shipping/consumer and remaining target/feature gates, a compatible Chitchat path
(the costly preview stays declined), root graph adoption and full product gates
remain open. Original all-writer retirement, durable completion/cells 2–4,
SIGKILL/lost-ACK and reconnection/host admission remain separate release blockers.
