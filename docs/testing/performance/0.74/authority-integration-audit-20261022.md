# Actual authority journal integration boundaries

The offline audit defines the owned-storage integration boundary without editing
the shared fixture/fixed engine. A Leaf-local cache cannot be adopted alone:
every journal refusal must evict all temporary buffers, and the current observer
and historical guards must not be mistaken for candidate equivalence checks.
The [preregistered audit](authority-integration-audit-plan-20261022.md) and
[derived facts](local-runs/authority-integration-audit-20261022/facts.json) retain
the exact source identities. Audit implementation: `196920c4`.

## Ownership and refusal paths

Original File, stamp, name and expected raw bytes remain independently owned by
each Leaf. The entries Vec retains previous generations during append. Reopen
rebuilds all original entries, verifies history and external pins, then performs
fresh validation. Fixed, signed and trust-input layers keep borrowing or wrapping
the original journal; a new cache must not introduce another authority owner.

The five failed assignments require whole-journal scratch eviction:

| Site | Failure boundary |
| --- | --- |
| External latch | Fixed/signed/trust refusal reaches the original journal. |
| Validation | Either outer directory/inventory bracket or any record/marker fails. |
| Inspection binding | Fresh reads pass but independent policy/host binding fails. |
| Operator transition | Change revalidation passes but the transition is invalid. |
| Publication | Preparation, write/seal/sync, entry opening or closing validation fails. |

Clearing only the current Leaf would leave caches in unvisited or previously
successful entries. The future patch must centralize failure handling, clear
every record/marker scratch, retain original history and refuse later IO.
Explicit data refusal remains distinct from a failed owner. Normal drop still
unlocks the directory and retires owned storage; no clearing of original evidence
or silent reopening is permitted.

## Validation and durable publication stay unchanged

The real change method validates before transition evaluation. Successful change
then enters publication with its own opening and closing rounds. Each journal
round retains directory/inventory checks before and after ordered record/marker
reads. A cached buffer may replace allocation only: fresh open, held/named metadata,
ownership/mode/link checks, parent device, bounded EOF read and original byte
comparisons stay in the same order. The fresh File must survive those comparisons.

Publication retains three explicit directory sync sites and two write_seal calls.
Each seal keeps write_all, permission change to 0400 and file sync_all in order.
RecordCreated, RecordSynced, MarkerCreated and DurableCommit checkpoints keep
their boundaries; the new Entry and Record are appended only after DurableCommit.
This is a source-order audit, not a new fsync or real-crash experiment.

The clean model gate assumes prechecked policy/context. Actual Record::advance
also checks refused state, a higher epoch, unchanged machine and issuer, a changed
policy digest, revision budget and the previous canonical-record hash. Storage
integration must preserve those real predicates rather than transplanting the
model's restricted transition helper.

## Observer and historical guards need separate identities

Fourteen named source-guard files are pinned in the audit: current whole-runtime
identity checks, operator ordering, observer token restoration and historical
normalization chains. Their current refusals on a runtime patch would be valid.
The audit does not weaken them or claim a complete compiler dependency closure.

The existing observer accepts &Leaf and iterates immutable entries. Its traced
read reconstructs the old allocating body, and its token guard compares that
body with the current runtime. A mutable cache cannot pass this boundary merely
by changing a reference or letting an old control observe a different body.
Historical readers also pin current source bytes. Redirecting only those guards
to a snapshot could hide an unverified candidate.

The next implementation therefore needs two explicit views: immutable historical
source/evidence replay, plus a new current-candidate observer and exact-body guard.
Keep old readers, controls, first results and snapshots unchanged. The candidate
guard must cover the storage substitution, mutable receivers, all five cleanup
paths and every preserved validation/publication body. Unknown or unrelated drift
must still refuse. Old measurement images must not be executed for this migration.

## Next implementation and remaining admission gates

Prepare one minimal runtime candidate: optional owned scratch per Leaf, cold
constructors/reopen, explicit mutable entry iteration, and centralized complete
error eviction. Keep raw/stamp/File ownership, actual Record transitions, syncs,
checkpoints and live schedule untouched. First implement the historical/current
guard split and candidate semantic controls; then compile and check real temporary
filesystem behavior, including partial tails, post-append errors and external
latches. A source audit or synthetic cut cannot substitute for those checks.

Twenty-three prior seals/1363 members preserve the isolated model result and the
older broad-run isolation refusal separately. This audit adds no model execution
or measurement. Runtime, qualification, original 300/60/10 windows and first
Causal refusals 10042390247/10010835739 ns are unchanged. No WSL, clocks/services,
server/install/supervisor or product action occurred. Authenticated delivery,
all-writer cessation, full SIGKILL/lost-ACK, reconnect and server admission remain
independent gates after storage integration.

All 22 first local verification steps passed: 109 offline controls, 79 source
guards in 30 binaries, format/check/strict clippy, documentation, 186 registered
gates and the local non-promotable performance contract. The initial CLI output
directory prerequisite failure is retained in the packet development note; no
facts were overwritten and no test/model workload was retried to change a result.

Inventory SHA-256:
`6f64406d00d59a0d55fd4a00994af3ae9a2aebcbeb2c2eaf5ed05e1df283cfea`.
It covers 60 members, excluding inventory.json; all 61 packet files had identical
raw working/staged bytes. Sources are retained under method/ and the completed
documentation checks have a separate receipt. Evidence commit: `bce13b30`.
