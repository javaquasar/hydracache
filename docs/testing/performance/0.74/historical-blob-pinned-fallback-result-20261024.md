# Historical Git fallback output binding

The isolated native candidate now refuses the demonstrated ABA output drift.
Fallback output must match an independently prepared reference bound to the exact
query, original object/missing pin and replacement policy. Restoring refs/config
after fallback cannot authorize newly returned bytes. This closes the component's
unpinned-output gap; **real caller admission remains false**.

Code commit: `930b82b2ee5216f904dbc0e4aea6c94826c43212`.
The [candidate evidence index](local-runs/historical-blob-pinned-fallback-20261024/index.json)
retains first sources, raw outputs and check results. The earlier
[fault report](historical-blob-fault-result-20261024.md) and its immutable packet
retain the original unsafe result, fixture failures and deadline refusals. This
candidate does not replace those historical verdicts with a green result.

## Binding contract

`FallbackPin::from_reference` stores the raw query, a structural copy of its
original decoder pin, explicit default/disabled replacement policy, terminal
status, and separate stdout/stderr lengths and SHA256 digests. It uses existing
`sha2`; there is no dependency or protocol manifest change. The constructor
asserts a reference, not trusted production provenance. All bindings are supplied
before reading and remain immutable during the operation.

`read_pinned` and `read_pinned_observed` validate positional bindings before Git.
The batch still checks every original object/missing pin. A non-blob/missing item
still uses the original `show` query and replacement policy; its owned result
must then match the original output pin before publication. Hashing consumes the
same deadline, with checks before and after verification. Earlier results stay
private until the entire read succeeds.

| Condition | Refusal |
| --- | --- |
| Raw-pin-only entry or positional `None` for a missing/non-blob item | `FallbackRequired`, before Git/observer effects |
| Wrong count, query, original pin, replacement policy or inappropriate blob binding | `FallbackIdentity`, before Git/observer effects |
| Changed stdout, stderr, stream length or terminal status | `FallbackOutput`, no result prefix |
| Reference/aggregate output exceeds the original stream budgets | `Budget` |

The old `read`/`read_observed` entry points retain blob-only operation but can no
longer execute an unpinned fallback. Reference statuses outside 0..255, missing
terminal status and a success status for an absence pin refuse enrollment. No
fallback is retried, no pin is refreshed from observed bytes, no short revision
is expanded and no replacement policy is silently disabled.

This contract proves the returned byte outcome belongs to the supplied reference
pair. It does not prove repository configuration stayed constant continuously,
that the pin producer is trusted, or that arbitrary existing-loader environments
are compatible. Those remain separate admission questions.

## Fault and regression outcomes

The original missing-path ABA uses the same three logical cuts and mutation
sequence, with an earlier blob added to exercise no-prefix publication. Default
policy now returns `FallbackOutput`; explicit replacement disabling retains exact
baseline bytes and diagnostics. Subsequent absence reads still match their old
reference after ref restoration.

Two additional real-Git cuts refuse with `FallbackOutput`: replacing a validated
tree just before rendering, and toggling local `core.useReplaceRefs` around
rendering of an already replaced tree. In each case the ref/config is restored
before output verification. Disabled policy survives the specific tree-ref
mutation. The old `core.quotePath` negative control still passes unchanged
rendering; a harmless config change is not mislabeled as output drift.

Pure controls reject per-stream byte/length changes, status changes, absent
terminal status, swapped streams, binding identity changes and aggregate budgets.
Duplicate fallback order is retained. A later wrong stderr pin publishes neither
an earlier blob nor an earlier successful fallback. Expiry at the first observer
cut still returns `Deadline` without a new grace period.

Rust 1.94 passes 9 decoder plus 5 fault cases, then all 6 transport cases in a
separate sequential command. Rust 1.88 passes all three targets in one command.
The transport corpus still compares all 54 exact independent-reference outcomes
across six full/short/replacement-policy profiles, including missing/tree results.
Owned-child timeout/output-overflow controls remain green and do not imply
descendant or FD-alias retirement authority.

Current all-target check, all-target/all-feature strict Clippy, focused rustfmt
and 31 Python regression cases pass. There is no measurement run, deadline
increase or retry-to-pass in this candidate. First source snapshots also retain
the initial compile version before the failing-batch fixture was supplied with
the new binding prerequisite; that fixture still tests its original `Process`
refusal rather than failing at the earlier new preflight.

## Remaining admission boundaries

The pinned-output component gate is green; production pin provenance, actual
caller error/environment and fresh-observation compatibility, supported Linux
proof, and the cost of preparing/hashing/spooling references remain open. The
old interrupted whole-package gate is not rerun or completed. No throughput,
CPU or memory improvement follows from this correctness result.

All old live predicates and their observation order/frequency, original
300/60/10 windows, the 780-second package guard, qualification manifest and frozen
0.73 artifacts remain unchanged. No clocks, services, server, original-owner
attempt, product workload, installation or qualification is used. The next step
is to establish an independently trusted pin producer and one exact caller's
eligibility before integration, not to treat this component pass as enrollment.

The subsequent [caller/Linux audit](historical-blob-caller-linux-result-20261024.md)
confirms existing census test-source anchors and local GNU component controls.
It also retains a real successful-stderr compatibility counterexample, so caller
admission remains closed. This is not production pin enrollment or clock admission.
