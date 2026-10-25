# Held Windows lineage controls

The external attribution adapter now has a bounded lineage join from an owned
root through up to four descendant levels. One native synthetic session passes
both positive shapes and rejects a wrong full executable path. It does not
launch Cargo, repeat the 321-case target or finish the full-package gate.

## Attachment and counter ownership

`process_lineage.py` extends the unchanged
[owned-process collector](owned-process-observer-result-20261025.md). It takes a
process-only Toolhelp snapshot, selects one shape from fixed image-path policies,
and opens each descendant with limited-query and synchronization rights. The
original owned root handle is borrowed, not reopened or closed by the join.

For every edge, the parent must remain live on its original handle before and
after the child's attachment. PID, full executable path and creation identity
are checked; a child older than its held parent refuses. A second snapshot must
select exactly the same unique chain, followed by another live frame on every
held process. Ambiguity, changed edges, missing nodes, duplicate PIDs, cycles,
API errors and exceeded budgets refuse instead of selecting a replacement.

The finite policy admits at most four alternative shapes of one to four edges.
Snapshots have a 32768-row cap and candidate frontiers a 64-path cap. Snapshot
handles always receive cleanup, and enumeration errors other than
`ERROR_NO_MORE_FILES` are not treated as successful EOF. The implementation uses
the official [process-entry layout](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/ns-tlhelp32-processentry32w),
[process-only snapshot](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot)
and [enumeration completion](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-process32nextw)
contracts. No modules, heaps, threads, command lines or environments are queried;
machine-wide snapshot rows are not written to the evidence.

After attachment, only the held leaf supplies lifetime CPU/I/O/memory samples;
there is no PID reopening or continued process census. Samples are capped at
800, reject changed creation identity and regressing counters, and latch the
first refusal. Closing walks owned handles in reverse order, attempts every
close, and keeps cleanup failure separate from the original attachment error.
This is parent-only accounting, not a sum of short-lived descendants.

This diagnostic join is not a security authority. Toolhelp's parent identifier
does not attest original caller credentials, hostile parent spoofing, argv or
the mapped image's bytes. Matching the full path cannot prove the on-disk file
is the mapped executable. The result must not become a production original-owner,
all-writer cessation, release capability or permission to refresh trusted pins.

## Finite native controls and first outcomes

`lineage_control.py` runs exactly three fresh synthetic cases, stopping on an
unexpected failure: a two-edge chain, a three-edge chain, and a two-edge negative
control with the right basename but a foreign full executable path. Each child
is the isolated local Python fixture, not Cargo or a product binary. Every node
has an eight-second local deadline, owns/reaps only its direct child, and responds
to a fixture release file. The external harness has a five-second readiness
deadline and attaches once, with no search-after-refusal or retry-to-pass.

The first native session passes all three cases. The positive cases retain three
and four process identities respectively, with nondecreasing creation times and
one final leaf record read after the root has reaped its descendants. The negative
case preserves `held lineage image/PID mismatch`, receives no admitted lineage
and still cleans up successfully. All expected node output lines match, all root
exit statuses are zero and no cleanup error is recorded. These are native API
and semantic controls, not CPU overhead or performance measurements.

The first focused lineage model check passes twenty cases. The first complete
suite then reports 108 passed and one failed out of 109: the mock expected an
8.3-spelled temporary path while the runner correctly returned its resolved long
path. Only that expectation changes to `out.resolve()`; no executable comparison
or runtime guard is relaxed. Two more closing-frame/root-state controls bring
the final attribution suite to **111 passing tests**, including **28 new cases**.
The first failure, pre-fix test bodies and exact native producer source remain
in [the evidence packet](local-runs/held-lineage-20261025/index.json). The native
session is not repeated after correcting the unit test.

Code commit: `18c8edecd93f75856b173d516ff70d4c77d55d79`.
The primary evidence index has SHA256
`04a7c1082fbe307cbefa5952febe2a4d7f9ad912cee1a874df7f623b079a6fd4`
(48 members, 37 objects). The
[supplemental gate packet](local-runs/held-lineage-gates-20261025/index.json)
has SHA256
`50cb5383ca8d5be41568a144d85e13fd0734c88667c845ab7d350c357a8e81bc`
(30 members, 20 objects). Its independent staged verifier checks raw hashes,
native source joins, both positive chains, the negative result and the preserved
first model failure without invoking any producer.

The 91 historical-tool cases also pass, for **202 Python cases** in the two
final suites. Compileall, tabnanny, unchanged v3 source/environment preflight,
doc-check and the local non-promotable performance-contract check pass. Each
bounded command retains unchanged source/HEAD receipts. This Python-only step
does not relabel earlier Rust compilation/lint results as new runs.

## Next Cargo adapter gate

Before any target dispatch, bind the actual original wrapper, Cargo shim and
toolchain image, and exact built test image to an independently reviewed launch
receipt. Freeze accepted shapes and source/build identity rather than learning
trusted paths from the discovered process. Preserve the original Cargo command,
CWD, inherited environment/streams, four test threads and 780-second origin.

The coordinator still needs bounded discovery and a first-refusal path that
leaves the original wrapper's deadline in control. It must not kill the target
early, retry it, splice another target's results or claim descendants' CPU from
the held leaf. No new Cargo dispatch is exposed by these modules.

All original 300/60/10 windows, live observation count/order/frequency, Rust and
dependency sources and qualification identities remain unchanged. There is no
server, service, clock, product, installation or qualification operation.
