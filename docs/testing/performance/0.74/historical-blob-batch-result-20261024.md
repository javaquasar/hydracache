# Historical blob batch prototype controls

The isolated Python batch reader passes twenty local semantic controls. In one
synthetic comparison, thirteen historical lookups (ten distinct commit/path
keys and three duplicates) use one batch invocation instead of thirteen separate
reference invocations. Every requested result remains present and ordered. This
is an invocation-count control, not a measured speedup or an installed Rust
optimization. Existing source guards and live checks remain unchanged.

## Candidate and reference boundaries

Code commit `840e4b8306b4b3825b95072c3d9dfeee0960cc32` adds only
`tools/historical-blob-batch-074/{reader.py,test_reader.py,README.md}`. Commit
`2707ddea8bca9507e571eeacd139f814ee103c36` adds closing deadline refusal and its
control. Neither modifies a Cargo package, dependency, current-file read,
original execution path or qualification input.

The candidate accepts a finite list of full lowercase SHA1 commits and explicit
UTF8 relative paths. One local `git --no-replace-objects cat-file --batch`
process returns commit metadata and the complete requested objects. Commit and
object types, names, content hashes, lengths, terminators and complete output
consumption are checked before publishing results. Duplicate source requests are
not deduplicated; only repeated commit metadata queries are grouped. Raw bytes
are neither decoded nor newline-normalized.

The reference is `git --no-replace-objects show <commit>:<path>`. Missing paths
and non-blob objects invoke that exact reference command, preserving stdout,
stderr and status. Invalid commit identity, batch diagnostics, process failure or
protocol corruption refuses the whole call without a retry or partial return.
This is not universal `git show` compatibility: short/dynamic revisions, arbitrary
flags/configuration, SHA256 repositories and non-UTF8 paths are not admitted.

The Git/parsing phase has one shared thirty-second diagnostic deadline, including
fallbacks and closing validation; cleanup has a separate five-second wait.
The input/request/stdout/stderr budgets are 256 KiB, 256 requests, 16 MiB and
64 KiB. Temporary files bound in-memory capture and avoid pipe-drain deadlock.
Sampled file-size refusal is not a kernel disk quota. Repository-overriding
environment refuses; child-only `GIT_NO_LAZY_FETCH=1` prevents partial-clone fetch.
These are explicit prototype boundaries, not a silent change to existing loaders.

## Semantic controls and retained first failure

Disposable synthetic repositories compare the candidate with the reference for
empty, binary, CRLF, Unicode-path, 256-KiB and symlink objects; multiple commits;
ordered duplicate requests; missing paths and tree fallback; replacement refs;
moving HEAD; current-file independence; separate repositories; and concurrent
batches. Protocol/mocked-process controls cover invalid requests, injection,
wrong commit, corruption, truncated/extra output, budgets, shared and closing
deadlines, process failure and exact-child cleanup. They run no product workload.

| Control version | Result | Disposition |
| --- | --- | --- |
| Initial candidate | 15 passed | First result and input digests retained |
| Extended controls v2 | 18 passed, 1 failed | Moving-HEAD assertion misplaced into symlink case |
| Corrected controls v3 | 19 passed | Fixture placement fixed; reader bytes unchanged from v2 |
| Closing deadline control | 20 passed | Late final fallback refuses before returning a result |

The v2 failure compared the correct symlink payload with the unrelated binary
fixture's expected bytes. Its exact sources and first output remain retained;
v3 restores the assertion to its original test without changing the reader.
The final closing-deadline change is separately committed, rather than treating
the earlier passing suite as proof that late completion cannot be published.
Compilation and indentation checks also pass. No new Rust/Clippy or full-package
pass is claimed for these standalone Python files.

The [evidence packet](local-runs/historical-blob-batch-20261024/index.json)
retains 46 logical members in 32 content-addressed objects: all eight check
phases, exact failing v2/corrected v3/final sources, coordinator sources and
installed Git documentation. Index SHA256 is
`17e63b99f997f7b9808ed2b1ce188dc0678294f52642fe4a7bd7c69f84eac4de`.
The initial fifteen-control version has its source digests and outputs retained,
not a complete source snapshot. Source and HEAD remain unchanged during every
check phase; the failed v2 result is not overwritten by a later pass.

## Integration decision

This closes the extracted protocol/semantic prototype, not package or production
admission. The next step is an isolated Rust reader with the same controls and a
call-site eligibility ledger covering exact flags, root/environment, revision
identity, error behavior and fresh-observation order. Only eligible immutable
lookups may change. Existing loaders without replacement disabling cannot simply
be redirected to this prototype, and the large short-revision cohort remains
uncovered. Full-package verification must still fit its unchanged 780-second bound.

No CPU, allocation, retained-memory, parser-cost or throughput improvement is
measured here. The earlier [static Trace2 observation](static-contract-git-attribution-result-20261024.md)
supports testing process amortization, not the size of its eventual benefit.
The [release closure boundaries](release-closure-audit-20261024.md), prior first
failures, ReservedLive frequency/order and original 300/60/10-second windows
remain intact. No server, service, clock, original-owner, installation,
qualification or push action occurs.
