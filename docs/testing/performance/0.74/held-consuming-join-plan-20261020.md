# Consuming original placement and retained held roles

Baseline 6e709b5da057e3e94bb12646e4d47071f8fb7750. Inherits docs/RULES.md.
Consume the exact FixedPlacement, PinnedRoles and fixed HeldTrustInput in one
non-cloneable, non-Send crate-private owner. No original, FD, checked policy,
start/send/replay/completion/unlock extractor. Failure retains every consumed
object; Drop retires original owner/root controls before returning borrows and
before dropping held role descriptors or the new trust source.

Reuse FixedPlacement::revalidate unchanged before and after each new round.
It retains fixed-path/root/guardian observations and the complete original
manager/consent/storage/clock frame. Do not replace any live predicate, reduce
observations or renew any of the original 300/60/10 origins/deadlines/46 brackets.
Perform two constant-depth rounds, checking new sources and full role content
between opening/closing full frames. Closing original clocks follow hashing.

The new private held-trust adapter reuses the existing original borrow guard.
It freshly checks policy/receipt/v4 under independent fixed source selection,
matches exact retained signed v3 bytes, and retains actual original request and
active authority revision/epoch/policy/trust snapshot across rounds. Signed claims
never select trust or a replacement original. Bounded document inputs are copied
only after all budgets pass. No new wire/durable format or production enrollment.

The private role adapter compares the complete consumed VerifiedBuild statement
with the freshly verified held receipt, not only image hashes, then reuses the
same PinnedRoles full descriptor/content/stamp revalidation. Wrong provenance
with identical images must fail. Neither adapter exports descriptors/originals.

Add three named runtime files and exact module hooks in fixed placement, held
runtime trust and role files; preserve all 443 old source bodies after those
hooks. Keep COMPAT, registry and qualification unchanged unless a new manual gate
actually requires registration. Preserve earlier sealed evidence.

Retain baseline and expected missing-module refusal before implementation.
Test exact order, consuming signatures, retained failure/drop order, every
opening/closing cut, all modeled original leaf refusals, cross-round drift,
budgets, sticky refusal and seeded substitutions. Exercise actual original
kernel/authority inputs and actual sealed role FDs with synthetic keys/temp roots.
These probes are not a privileged original FixedPlacement end-to-end rehearsal;
record that separately if its prerequisites are not available locally.

Run GNU 1.94/MSRV 1.88 repeatable source/image-bound tests, old controls and
capability docs, strict Linux/Windows lint, source guards, fmt, registry/docs and
local contract. Document confirmed correctness separately from unmeasured cost,
privileged full-path/SIGKILL/lost-ACK and production admission gates. No server
access/installation, supervisor operation, product or expensive workload.

## Candidate result and open gates

Implementation d9d37a8a; final tested code source f07dcca3. Three new runtime
files retain all 443 previous files after exact hooks. The private consuming
owner, complete original-frame calls, receipt-statement/role-FD binding, sticky
failed custody and drop order are implemented. Both compilers pass two rounds
of ten new models/five previous fixed models, eleven capability docs and strict
lint. Exact source guards and Windows/repository checks pass.

The complete local Linux join gate is NOT passed. Failed unbound, timesyncd-stop,
held-session and CPU-affinity cohorts remain in the
[evidence](local-runs/held-consuming-join-20261020/README.md). Traced later samples
show clock-order refusal; other samples lack an exact original error/site and
remain unclassified. No clock source, assertion, predicate or deadline is changed.
The authorized local timesyncd operation restores active state in finally.

Next resolve reproducibility of original kernel/time observations on a stable
Linux context or with a separately scoped clock investigation, then exercise
the privileged complete holder and attribute its four-frame/full-hash cost.
Only after those gates can original delivery, all-writer lifecycle and joined
SIGKILL/lost-ACK/server-readiness work advance. This plan is not marked complete.

The separately authorized [isolated server comparison](held-consuming-server-plan-20261020.md)
now passes the unchanged 17/26/5/38 selections twice on both compilers with
unrestricted CPUs. The original WSL refusals remain sealed and unresolved.
Stable Linux reproduction of these selected bridges/models is established;
the actual privileged holder, real role hashing cost and joined fault gates
remain open. No production candidate or installed supervisor was changed.
