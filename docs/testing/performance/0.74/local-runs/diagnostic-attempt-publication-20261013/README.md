# Local owned attempt publication transfer evidence

Implementation `6363ab7c80efd2af6524a153f56b4ab8db9d12b7`, tree
`35e822933bebb2390e3b8d353934da5e5fa42650`, transfers the original runtime-acknowledged
attempt to initial Reserved publication. Preregistration is
`4044d0b6f13370b08bd27d2b6060f83c756db06c`; baseline is
`89fdbd9540c9aed8a3cacd9af9defc52c9ba3403`. See the
[local transfer design](../../diagnostic-attempt-publication-local-design.md).

## Original objects and lock continuity

The fixed-only API consumes the opaque early-attempt guard by value. It moves
the original sealed attempt descriptor/stamp/bytes and existing fence into
publication storage, never releasing/reacquiring the lock or adopting a file
found by name. Full-state mismatch retains the healthy supplied guard and lock
without mutation. Bound failure retains original objects, borrow, first error
and safe-authority refusal uncertainty until drop.

The namespace admits the attempt only while its original document is owned.
State creation remains exclusive 0600 with existing schema-one/sync barriers.
Attempt remains 0400. Existing cell-one consumption inherits and revalidates it
alongside its original record/commit files. Legacy publisher/context checks
still refuse an unowned attempt filename. No durable schema, filename, state
advancement, execution or recovered acknowledgement is introduced.

## Functional and signal matrices

The fourteen fast cases cover successful transfer/cell-one continuation,
unbound mismatch, eight logical publication interruptions, late target/context,
attempt and original-intent drift, already-refused inputs, unsafe/advanced
authority, partial refusal, concurrent chains, legacy refusal and rejection of
unacknowledged internal storage.

Two ignored/manual Linux phases test actual SIGKILL: nine attempt-to-Reserved
cases and fifteen inherited-attempt cell-one cases, each with a separate
uninterrupted control. Seed `0x074d` permutes each full matrix. Every case checks
original lock contention/release, exact owned helper termination, retained
bytes/metadata and two fresh read-only processes. At publication BeforeCreate,
the acknowledged attempt is already retained and fresh reservation refuses.
AfterReturn death loses runtime acknowledgement despite surviving valid bytes.
The full process regression reruns all previous phases through the extended
private harness.

All fourteen functional cases pass in each of three exact-source repeats.
Each process repeat passes twenty-four SIGKILL cases and two controls, with
fifty-two fresh-process inspections. Full process regression passes six manual
tests, covering fifty-seven signal cases and five controls across all phases.
Linux scoped regression passes 633 with seven manual/host cases ignored;
Windows passes 204, root contracts/registry/governance 164, and Linux compile-fail
doctests 43. Current/MSRV checks, warnings-denied Clippy/Rustdoc, format and docs
checks pass. Qualification dry-run and ship admission still refuse. This is
not full workspace/CI release verification or installed-host evidence.

## Negative observations

The absent-entry compile result is retained before implementation. Clippy then
rejected a 256-byte internal Err variant; boxing the preserved original storage
on that failure path keeps ownership unchanged without a lint waiver. An old
structural contract required the explicit `only_state` call spelling. It remains
explicit in the legacy branch, with no relaxation of its actual refusals.
Neither negative capture is a successful final proof or performance measurement.

## Read only verification and next scope

Run:

```text
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-attempt-publication-20261013/verify-packet.py
```

The verifier binds source/tree, named repeats, raw logs, both manual gates,
W11 registry, historical raw evidence and all unchanged frozen qualification
inputs. The previous [early-attempt packet](../diagnostic-early-attempt-20261013/README.md)
remains historical proof for its exact source, not a current-source claim.

P1 still needs fixed cells two through four, signed next-state/history rules
and incomplete/reboot handling. Trusted peer/time/issuer inputs, execution,
watchdog/writer cessation/cgroup cleanup and installed-host rehearsal remain
P2 through P5. Product candidate/qualification/ship gates remain closed.
