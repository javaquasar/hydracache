# Held unit projection from the consumed owner

## Scope and result

The consumed `JoinedHeldPlacement` can now become an opaque `ProjectedHeldUnit`.
Implementation commit: `99ed157e`.
It retains the same placement, role files, trust input, original manager/message
owner and guardians. Success exposes only revalidation. Failure retains the
same refused holder; neither outcome exports a unit specification, message,
descriptor, role file or retry owner. Transport, release and execution remain
closed.

This is an exact **semantic projection**, not a materialized launcher or runnable
`TransientUnitSpec`. The distinction matters: the original owner has stdout and
stderr but no dedicated stdin pipe, authenticated release endpoint or sealed
launcher bootstrap. Its installed config is not the sealed config required by
the launcher. These future objects are explicit typed roles, not invented
descriptor numbers or evidence that they already exist.

## Binding and unit semantics

The only constructor consumes the joined owner without accepting arguments.
It obtains the signed placement from the retained documents, role identities
from retained sealed files, worker/config identities and digests from the
original bundle, and stdout/stderr identities from the original pipe writers.
Guardian PID and pidfd-object identities come from the same consumed, Armed,
non-refused root pair, not from the collector/custodian PID claims. The complete
existing validation authenticates these sources; JSON decoding by itself is
not authority selection.

The context retains original state and boot, owner credentials, four origins
and bounds, descriptor inventory, custody descriptors, addressed-message aliases,
manager/client names, serial, root binding and all five document digests.
Slice/root/unit names derive from the admitted attempt and cell. The selected
absolute deadline must remain the minimum of the three original bounds and
equal the root enrollment deadline. No clock is reset or admission renewed.

The projection preserves diagnostic hardening and limits. It replaces the
legacy direct-worker command with a typed retained launcher image and one
fixed argv element. The future worker command remains the fixed executable,
`--run`, `/proc/self/fd/5`, original worker hash and frozen source commit.
Environment and writable paths are empty, working directory is `/`, and file
size limit is zero. Legacy file-append stdio and `StandardInput=null` are absent.
There is deliberately no transport-ready ExecStart/path transformation.

| Launcher slot | Exact semantic source |
| --- | --- |
| 0 | Dedicated empty read pipe, not yet materialized |
| 1 and 2 | Original stdout and stderr writer identities, in that order |
| 3 | Authenticated original release endpoint, not yet materialized |
| 4 | Retained original worker identity and digest |
| 5 | Future sealed copy of the retained original config, with identical bytes |
| 6 | Original creator pidfd role |
| 7 and 8 | The two original Armed root guardians and their pidfd identities |
| 9 | Original absolute alarm at the minimum retained bound |
| 10 | Future sealed 256-byte bootstrap required by the existing launcher |

The role map matches the launcher protocol but does not assign its missing
kernel objects. Bootstrap token, actual new-object identities, owner peer,
pidfd/stream inheritance and an expanded exact descriptor inventory must be
materialized and checked together before a later transport implementation.
No existing signature or legacy message is silently upgraded.

## Validation and refusal

Projection creation and revalidation each call the unchanged consuming-holder
validator twice, with derivation and comparison between them. Each call still
executes both `frame → trust → roles → frame` rounds; no prior guard result is
cached and no intermediate live observation is removed. Closing projection
must equal opening projection; later revalidation must also equal the retained
projection, including every property and semantic slot.

Any validation error, changed context or projection mismatch retires the same
owner. Its existing owner-first retirement, independent guardian obligation,
descriptor destruction and fence behavior remain unchanged. Ordinary Drop of
the new wrapper drops that retained holder; no new terminal/disarm token exists.
The original 300/60/10-second windows and their absolute origins remain intact.

Four exact module hooks are the only changes in prior runtime files. The new
source canary removes only those hooks and compares the complete predecessor
files. Historical source views use this exact adapter, not an exclusion for
new predicate or transport bodies. Existing qualification, schemas, registry,
original live fixture and old evidence packets remain unchanged.

## Next boundary

Next materialize the typed missing roles under this same owner and extend the
exact FD inventory, then prove the original-owner projection in its unchanged
live windows. Only after that can durable one-shot delivery uncertainty,
authenticated original transport and actual process/barrier binding be joined.
Active collection, closure of external writer copies, terminal fence proof,
completion/cells 2–4, reconnect, enrollment and server admission remain separate
gates. No server, clock, service, installation, supervisor or product workload
is part of this step.

## Local tests

The [evidence packet](local-runs/held-unit-projection-20261021/README.md) records
10 new unit/property/API controls on each compiler. They check the exact role
map, command and hardening, both unchanged validator calls, every one of the
16 frame/trust/roles callback cuts, sticky refusal, closing drift, substitution
of each property and FD role, and 256 deterministic context substitutions with
seed `0x074051a7`. Read-only role, bundle and guardian views have separate local
controls. These model tests do not impersonate a live original-owner fixture.

Both compilers also pass 10 existing consuming-holder controls, 12 reply
state/flag/exchange controls and 38 synthetic held build/role controls: 70 tests per
compiler in total. Nine Windows source-contract controls pass, including the
462-file historical runtime/fixture reconstruction and exact hook/closed API
canaries. No new privileged fixture, original send, launcher execution or
SIGKILL/lost-ACK run is included. There is no performance measurement or claim.

GNU 1.94 and MSRV 1.88 pass affected-package `cargo check --all-targets` and
`cargo clippy --all-targets --all-features -- -D warnings`, offline and locked.
Windows checks and strict lint cover both the supervisor and xtask. Formatting
passes. Source manifests agree before and after each lane; the qualification
manifest retains SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Doc-check, the registry of 186 gates and the local non-promotable performance
contract pass. These focused checks do not substitute for release qualification.
