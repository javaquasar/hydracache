# Offline preparation cost ranking

This reader ranks the retained partial preparation profile without another live
attempt. It verifies the exact upstream evidence/index, member bytes, measured
source manifest and eight current source files before analysis. It is not a
runtime guard, authority input or substitute for the failed preparation gate.

For every round slot it checks incoming calls plus roots against node calls,
outgoing direct-child wall against inclusive minus outside-children wall, and
root-time conservation. Slots must fit within their phase, and their per-kind
counters must fit the phase counters. Separate kernel/live/authentication views
are never added together. In particular, live `KernelContext` wraps kernel
`PacketContext` in the same function; its large time is not an additional owner.
Outside-children wall is not exclusive CPU time, and wall minus CPU is not IO.

The source-derived local-context graph describes successful `Probe::observe`,
not a new syscall trace. Its 88 metadata requests include 24 calls in
`PinnedDirectory::same`. Multiplying by the 4232 observed LocalContext spans
gives an inventory for the successful outputs phase, not direct measured syscall
counts or a CPU saving. Original source pins prevent applying that inventory to
a different implementation silently. Failure paths can short-circuit it.

`next-directory-protocol.json` prepares one independent hypothesis: change only
the fresh five-field directory projection at `PinnedDirectory::same`. The
candidate is not implemented or measured here. Before selecting an implementation,
audit the pinned GNU metadata syscall/fallback/error contract. Keep constructors,
documents, namespaces, all live frequencies/order and original 300/60/10 windows.
A successful leaf-only screen cannot replace complete bracket/context controls.

From the repository root:

```powershell
C:/Python313/python.exe tools/preparation-ranking-074/test_review.py
C:/Python313/python.exe tools/preparation-ranking-074/review.py
```

Both commands read saved data only. The first result, original refusal, missing
later phases and instrumentation overhead stay visible. No bootstrap resampling,
cross-image timing comparison, performance improvement or admission is inferred
from this one partial profile. JSON output is diagnostic only.
