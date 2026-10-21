# Remaining fresh check leaves in ReservedLive

The safest next isolated experiment is a pure allocation baseline for the small
machine-id/boot-id scratch in `worker_context::Document::read_with`. Its inputs
and refusal boundary are bounded; no policy, observation or owner representation
needs to change. Two other small candidates remain: short trust-component C
strings and collector peer-vector growth. This is a review order by scope and
risk, not a measured ranking of their exclusive costs.

The [audit packet](local-runs/reserved-leaf-audit-20261021/README.md) checks four
earlier raw seals and all 186 members, reconstructs the existing full round
facts, and narrows the retained trace to ReservedLive. It records seventeen
canonical source hashes and thirteen leaf witnesses, including the two most
recent allocation candidates already retained. No runtime or observer changes,
new measurements, WSL invocation, clock/service operation or server work occur.

The auditor and controls are committed at
`9e6b84a48c44880fbd3b53eb948be9b9a1b6479e`. The packet has 23 members plus its
inventory, whose raw SHA256 is
`d66631eec23dc969f372d73c2bb83519058214383023564ec269ec542f854c4e`.

## Keep source and measurement identities separate

| Evidence | Source and scope |
| --- | --- |
| First refused material traces | `1d6d179bd7150ef564560821a897794e20b24b36`; one original GNU and one MSRV attempt, not new samples |
| Current audited runtime | `e59190ffa431f36d9fc37998e60b368bcc2a1623`; contains the later pidfd pathname and Groups collection candidates |
| Later allocation controls | Pure pathname and collection operations in their separate sealed packets; neither measures current full preparation latency |

The old trace cannot establish current ReservedLive duration after either
candidate. Its span counts must not be multiplied by the pure control savings
to create a post-candidate allocation total or speedup. The two pure cohorts
also use different operation counts and cannot be added as one workload.

## ReservedLive alone in the retained trace

The earlier [round attribution](held-material-round-attribution-20261021.md)
ranked kernel kinds across all eight steps. Its roughly 71.4% Boot/Namespace/
Origin subtotal is not specifically a ReservedLive share. The newly retained
view reads the already recorded per-step kernel nodes; it collects no new data.

| Final combined phase ReservedLive component | GNU ns | MSRV ns |
| --- | ---: | ---: |
| Complete step wall | 2808784775 | 2941286486 |
| Kernel wall outside instrumented children | 1745205141 | 1828186002 |
| Other wall inside ReservedLive | 1063579634 | 1113100484 |

All 56 GNU and 59 MSRV ReservedLive calls complete. The later Causal check still
refuses the unfinished overall round. Kernel and other components sum exactly
to each step wall. Per-kind detail follows; these are nested wall observations,
not exclusive CPU or syscall counters.

| Kernel kind within ReservedLive | GNU calls | GNU ns outside children | MSRV calls | MSRV ns outside children |
| --- | ---: | ---: | ---: | ---: |
| Boot | 158144 | 509648998 | 166616 | 538980294 |
| Namespace | 316288 | 435065110 | 333232 | 450718663 |
| Origin | 79072 | 305941551 | 83308 | 317490680 |
| Document | 239680 | 152267807 | 252520 | 158280561 |
| Projection | 40768 | 148967856 | 42952 | 154730047 |

The facts preserve all ten kernel kinds, not just these five. The separate live
view includes LocalContext at 4480/4720 calls and 315612049/322677375 ns inclusive,
WorkerSource at 17920/18880 calls and 120188760/125365321 ns, and ActiveAuthority
at 18032/18998 calls and 421411078/448425499 ns. These live spans overlap kernel
and other live scopes; they cannot be added to the kernel partition or used as
exclusive allocation, authority-buffer or observer cost. No residual is renamed
as fsync, effects, memory initialization or canonicalization.

## Reachability and observation boundaries

Both complete validation and pre-consent frames route ReservedLive to the
original reservation's `live`, then `live_parts`. On a successful path,
`live_parts` captures and validates the original request, compares its encoded
reserved record, obtains original reservation time, verifies original collector
requirements, compares identities/times and captures the request again.

The worker time/collector helpers bracket their packet operations with the
same KernelWorker revalidation. KernelWorker still checks policy, original
packet context, local Probe, original packet context and policy in that order.
WorkerPolicy keeps trust-source checks around active authority and policy
verification. These relationships explain reachability; the audit does not
replace them with a flattened pass or infer a new global call multiplier.

Probe observation still has two iterations, each with a namespace bracket,
machine document read, boot document read and another namespace bracket.
`Document::read_with` performs retained metadata comparison, one positional
read, retained metadata comparison and then the length/newline/content checks.
There is no retry on short read or EINTR. This is a different leaf from kernel
Origin's already stack-backed Boot document reader.

## Remaining leaf inventory and review order

| Leaf | Current implementation and boundary | Disposition |
| --- | --- | --- |
| Local context document scratch | `vec![0; length + 1]`; production lengths 33 and 37, callback slices 34 and 38 bytes | First separate pure allocation experiment; preserve the exact slice length, one read and both metadata checks |
| Worker trust component name | `CString::new(name)` before the same flagged openat; the helper is reused by controller/trust readers | Second small candidate; preserve embedded-NUL rejection, arbitrary-name compatibility and owned fallback rather than assuming every caller is a fixed literal |
| Collector peer-vector growth | Supplementary `to_vec`, primary push, sort and dedup | Third small candidate; test empty/maximum memberships, primary present/absent and exact canonical union without changing the public Vec |
| Proc document scratch | One zeroed 65537-byte Vec per Sender revalidation, already reused within that invocation | Attribution and lifetime review first; do not move 65 KiB onto recursive frames, retain stale observations or add unsafe uninitialized reads |
| Sample boot ownership | `self.boot.clone()` after the same two fresh Origin rounds and two clock samples | Defer representation changes; Arc/inline identity is broader than a collection leaf and its exclusive benefit is unmeasured |
| Authority fresh leaf bytes | Fresh flagged open, metadata/ownership/budget checks, capacity from observed length, bounded read-to-end and closing stamp/length checks | Preserve the read/EOF/error schedule and both original/name identities; metadata length is not permission to skip reading or checking growth |
| Authority observed inventory | Fresh read_dir, owned names and BTreeSet, duplicate/count/oddness and exact expected-name checks | Broader semantic review; a compact recognizer must preserve the first error and all malformed/unexpected/duplicate/over-budget cases |
| Trust snapshot bytes | Fresh zeroed limit+1 buffer; bounded offset loop, existing interrupted-read retry schedule, fresh named and retained comparisons | Keep 2048/8192 limits, partial/EOF handling and retries distinct from the one-read local document leaf |
| Local context temporary descriptor | `current.try_clone()` feeds PinnedDirectory metadata/procfs checks in named ancestry | Descriptor-lifetime and error-order review, not a new FD cache or removal of named checks |
| Canonical request reconstruction | Repeated request cloning, hashes, serde decoding/encoding and canonical byte comparison inside capture/encode | Not one allocation hypothesis; isolate an operation before proposing any change, with exact bytes and full validation brackets intact |
| Fresh kernel traversal | New proc root and namespace objects, filesystem/metadata checks, typed namespace ioctls, boot ancestry and document reads | Preserve observations and ordering; span counts are not measured syscall counts, and syscall replacement requires a separate identity/error review |

The existing authority expected-name/fd pathname formatting, fixed kernel child
names, namespace literals and boot comparison already avoid their earlier
temporary ownership. The pidfd pathname and Groups growth candidates have their
own allocation evidence. They are not new remaining opportunities simply
because the old measured trace predates their implementation.

## Acceptance boundary for the next small experiment

First retain the original local-document allocation operation as a pure
baseline, without calling Probe or reading proc/sys files. Check canonical
lengths and the existing length-four internal test fixture; do not silently
narrow the generic private helper's accepted shape. Any proposed bounded stack
storage must pass exactly `length + 1` bytes to the callback, not a full 38-byte
buffer for machine-id. Test short/zero/overrun counts, malformed bytes, newline,
EINTR/EIO, and unchanged first-error order using synthetic callbacks/metadata
models, with no repeated original owner.

Only after that baseline should an isolated implementation and exact whole-file
reversal be considered. Successful pure allocator tests do not prove that the
315/322 ms LocalContext span was allocation cost, that the new stack branch is
faster, or that preparation now fits the original ten-second window. No such
implementation or live comparison is included in this audit.

Eleven new offline controls pass alongside the existing fifteen round,
nine material-reader and eighteen arithmetic controls: 53 distinct Python controls.
Nineteen existing source canaries pass. Windows xtask all-target check and strict
all-feature clippy, doc-check, the unchanged 186-gate registry and local
non-promotable performance contract pass. Protected runtime/source comparison
against the audited tree remains empty. The initial audit publication refused
a missing new output directory and wrote no result; the separate packet retains
that administrative outcome without changing an input or repeating a profile.

The first Causal(Time) ages remain 10042390247 ns and 10010835739 ns. Original
windows are 300/60/10 seconds; qualification SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The failed full material gate and missing successful cleanup/EOF/fence,
authenticated delivery, all-writer cessation, full SIGKILL/lost-ACK/reconnect
and server admission proofs remain independent. No timing claim or readiness
promotion follows from an offline leaf inventory.
