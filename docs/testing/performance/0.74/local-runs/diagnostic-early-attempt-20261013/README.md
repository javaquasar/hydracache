# Local early diagnostic attempt evidence

Implementation `cdeb2c8c8f308161644b9fc9bf224282da2c2dad`, tree
`d22853a826e2ac4ecac9ad9c99a9db05267a3f44`, adds the original early-attempt guard.
Preregistration is `525cda7121a0c545c024b4bc583e15cf0278be17`; baseline is
`277f774a4fabad2b3e76fa3f907554f51cc45d22`. See the
[local design](../../diagnostic-early-attempt-local-design.md).

## Admission and retained context

The only new document is `diagnostic-attempt-074.json`, exclusively created
under the original fixed intent/fence. It reuses the bounded canonical retained
intent schema, stores the exact signed initial attempt, seals the owned file
0400, and syncs before acknowledging. Any partial or complete retained context
blocks fresh empty-fence reservation. No Reserved state or cell-one record is
created. The original state/cell publishers remain unchanged.

BeforeCreate is an unadmitted attempt with no durable record, not consumed
nonce proof. AfterReturn SIGKILL loses the opaque runtime acknowledgement;
fresh read-only observation cannot recreate it. This is local retained-context
protection, not global nonce enforcement, host rollback protection, authenticated
peer/time, empty-cgroup proof or production readiness.

## Functional and process loss checks

The twelve fast cases cover canonical retention, unbound mismatch, eight
logical boundaries, late target/context, original object and intent drift,
unsafe/advanced authority, uncertain refusal, already-refused inputs, concurrent
independent chains, bounded input/replay and original-thread-only ownership.

The manual gate kills only the exact owned child at all eight boundaries and
after return, plus an uninterrupted control. Each case checks original lock
contention/release, retained inode/owner/group/mode/link/length/mtime/ctime/bytes,
owned helper termination and two fresh read-only processes. Seed `0x074d`
permutes the entire matrix. Separate regression reruns the original P0 matrix
using the extended harness. No broad process kill, server, workload or reboot is
performed.

All twelve cases pass in each of three exact-source functional repeats.
Each of three process-loss repeats passes nine SIGKILL cases and one control,
with twenty fresh-process inspections. The extended full process regression
passes four manual tests: twenty-four original P0 signal cases/two controls
and the new attempt matrix. Final Linux scoped regression passes 619 with five
manual cases ignored; Windows passes 204, root contracts/registry/governance
163, and Linux compile-fail doctests 41. Current/MSRV checks, warnings-denied
Clippy/Rustdoc, format and documentation checks pass. Qualification dry-run
and ship admission still refuse. This is not a full workspace release gate
or installed-host/CI qualification receipt.

## Retained negative observations

The absent-entry capture also exposed a test fixture that assumed Clone on
`DiagnosticPeer`; the following failed compile isolates that fixture mistake.
The fixture now explicitly copies asserted fields without changing the API.
Early provisional passing runs had unused fixture variables. Warnings-denied
Clippy rejected them; meaningful assertions/revalidation replaced the omissions,
with no lint waiver. The first root-contract run used a formatting-dependent
literal for the original fence call. Its check now matches the actual call,
without changing the storage algorithm. All negative captures remain in the
manifest and are not counted as successful final runs.

## Verification and next boundary

Run the read-only checker:

```text
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-early-attempt-20261013/verify-packet.py
```

The packet binds exact source, raw captures, named repeats, manual gate, W11
registry and unchanged qualification inputs. The preceding
[transition packet](../diagnostic-transition-20261013/README.md) is historical
evidence, not reinterpreted as verification of this changed source.

P1 still needs an owned attempt-to-Reserved transition and cells two through
four, signed previous-record history and reboot/uncertainty handling. A readable
attempt cannot be reopened, adopted or used to launch work. P2 through P5 and
qualification/ship permissions remain closed.
