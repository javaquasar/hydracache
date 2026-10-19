# Fixed leaf path allocation cost

Inherits `docs/RULES.md`. The separate wall-time experiment is negative: even
byte-identical images produce a significant small difference in one local
A/A comparison. Do not waive that experiment's timing rule or claim a wall
speedup. No runtime candidate has been implemented or accepted from those data.

This allocation experiment uses the same isolated path-construction proposal
but a different, explicit cost endpoint. Before implementation, add a test-only,
opt-in allocation observer and record three baseline A/A repetitions on each
GNU toolchain. Observe actual Rust global allocation/reallocation requests and
requested bytes for 1,024 fresh calls per leaf. Do not estimate malloc calls
from source. The observer is inactive outside its current thread capture,
preserves nesting/unwind, never allocates or panics inside allocator callbacks,
and is absent from ordinary test and production images. It confers no authority.

Adopt only if baseline A/A counts agree exactly, candidate and frozen reference
still return the same values/identities, both leaves reduce allocation or
reallocation requests on both compilers, no measured request count or requested
byte total increases, and all semantic/source/GNU retirement checks pass. If
not, preserve negative evidence and revert only the runtime substitution. These
are deterministic allocation-cost criteria, not a replacement latency estimator
or a relaxation of any deadline or qualification gate.

The only runtime proposal remains fixed C string boot children and borrowed
literal namespace paths, with unknown names retaining original formatting.
Keep every open, filesystem/metadata check, read, close, live observation,
46 effect brackets and original 300/60/10 origins and deadlines. No cache of
validation results, new original start/completion capability, product API change,
server access/installation, supervisor operation or product load is allowed.
After the decision run three fresh unprofiled GNU retirement-frame controls
and its eight-case regression. Profiling binaries are never retirement evidence.

## Outcome

The separate allocation endpoint passes at runtime source `0317f3dc` on GNU
1.94 and MSRV 1.88: three identical samples each show boot 5 → 1 Rust requests
and 62 → 36 requested bytes, namespace 1 → 0 requests and 42 → 0 bytes, with
zero reallocations and no counter increase. Baseline A/A and frozen-reference
counts agree exactly. The ordinary GNU candidate passes three retirement
callbacks and the old eight-case regression; the pre-candidate baseline also
passes. All 297 source contracts and scoped semantic checks pass.

The [closed packet](local-runs/fixed-leaf-path-20261019/README.md) preserves all
raw observations, failures and the scope of the local simulated-root gate.
The runtime substitution is retained for allocation cost only. No wall-speedup,
product claim, original privileged constructor or server admission follows.
