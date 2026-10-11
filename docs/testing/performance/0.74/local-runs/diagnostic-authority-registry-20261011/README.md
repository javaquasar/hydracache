# Local authority journal evidence

This is local safety evidence, not production authority or release qualification.
Baseline: `27e5f0bd7c7596a8e54d9720cdeebae06ea1d268`; preregistration:
`08fbf0d1f1a7c1c8d1a359f23f547e3a7e653f46`.
The manifest binds reviewed implementation/tree identities and exact raw captures.
The packet retains 54 captures, including the expected and unexpected negatives.
Initial implementation: `5e826dd7ed5998564c0c4581534df9fe027febca`; reviewed lock-lifetime fix:
`e777f5a6e5b16c724d63d6c139761d5d05e44e08`, tree `87164ff889feaf69b684c7e167505987b6409931`.

The canonical core enforces independent current pins, contiguous predecessor
digests, exact-binding refusal and explicitly higher-epoch reactivation.
The separate nonroot Linux /tmp fixture journal owns a nonblocking exclusive
directory lock and uses create-new record/marker files with file/directory fsync.
It does not overwrite, repair or evict. Bounds: 128 generations, 2048-byte records,
65-byte markers and 256 names. Only successful explicit refusal is persisted;
inspection/IO failure latches the current writer in memory. Active numeric views
are inspection data, never start permission.

Baseline and absent-API red are retained. Reviewed checks cover strict canonical
grammar, transition exhaustion, seeded mutation, real local close/reopen, explicit
refusal persistence, newly pinned higher epoch, exclusive writers, unsafe paths,
linked/special/oversized leaves, original object drift, partial-write ENOSPC and
four private logical publication faults. A complete marker after late failure
may be visible without acknowledgement; incomplete tails refuse, never replay.
Logical faults and grammar mutations are not physical power-loss or kernel races.
The Windows dead-code lint negative is retained; platform gating fixes it without
suppressing warnings.

Parallel review also finds `WouldBlock` after original-owner drop. A retained
descriptor duplicate reproduces the lifetime bug deterministically. Explicit
owner unlock fixes it without retry; closing the old alias cannot unlock the new
writer. The actual parallel alias is not identified, so no observed fork is
claimed. Both the parallel failure and deterministic red/green remain separate.

Reviewed results: 409 scoped Linux passes and one existing ignored; 195 portable
Windows package passes; 102 contract, 13 evidence and 23 governance passes.
Fourteen new unit tests (five portable core, nine Linux fixture) pass in three
serial repetitions; one portable API and one root contract check are added.
Seed: `0x7642026`, 256 modeled mutations. Both platforms pass strict supervisor
clippy and Rust 1.88 all-target checks; xtask strict clippy, Linux strict rustdoc,
formatting, doc-check, local non-promotable contract and book checks pass.
Counts are test executions, not performance samples or all-platform coverage.
Post-packet registry/doc/contract/book checks pass; 179 local Markdown file
targets resolve. This is not an external URL or anchor check. Four prior local
packets retain their original raw hashes and source/tree identities.

The stricter qualification input check remains red: the plan changed at
`8e16117b`, but its declared manifest digest was not reconciled. This mismatch
exists in the baseline committed LF content; checkout CRLF is a separate
difference. All eight inputs remain unchanged relative to that baseline.
The raw qualification manifest digest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No frozen identity is silently refreshed. The initial raw-input diagnostic
attempt has an empty log because its terminating exception was outside output
capture; `qualification-input-red.log` retains the actual checker refusal.

No full workspace verifier, exact product-candidate receipt, performance
measurement, physical power-loss behavior or regular-file IO deadline is claimed.
Whole-host snapshot rollback, root/owner tampering and historical signature
attestation are excluded. Root-owned fixed storage, original signed-guard durable
composition, real operator issuer/pins, production preparation/start and lifecycle
watchdog/writer cleanup remain open. No host, workflow, product workload or frozen
qualification input changed.

Read-only verification from the repository:

```powershell
python docs/testing/performance/0.74/local-runs/diagnostic-authority-registry-20261011/verify-packet.py
```
