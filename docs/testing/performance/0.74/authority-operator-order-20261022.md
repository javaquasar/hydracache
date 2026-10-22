# Authority operator order correction and refused test isolation

The separate operator candidate follows the actual engine's change/transition/
publish validation order for a restricted, prechecked state/epoch context. Its
observed correctness matrix matches the oracle, but **targeted test isolation is
REFUSED**: a PowerShell single-string splat accidentally selected all twelve
compiled tests instead of only the three new ones. The complete broad run is
retained, not relabeled as a compliant targeted cohort. Unexpected old allocation
rows are excluded from new claims; no image was rerun after the argv repair.

See the [preregistered plan](authority-operator-order-plan-20261022.md) and
[derived facts](local-runs/authority-operator-order-20261022/facts.json).

## Counterexample and scoped implementation

The first legacy-only controls ran before candidate source existed. Both MSVC
compilers agree on sixteen cases with epoch 7: six outcome-priority mismatches
and eight validation-round mismatches. Legacy Invalid is normalized to logical
Transition only for the modeled state predicate; raw outcomes remain recorded.
This is a passing falsification test of the old model, not conformance evidence.

The new helpers revalidate before evaluating a transition and revision budget.
A failed owner therefore returns Refused without IO; an initial read/bracket
failure takes priority over transition validity. A healthy invalid transition
observes one full round, and a successful logical publication observes change,
publication entry and publication exit rounds. Expected history/tokens remain
independent of temporary caches, and every modeled error latches failure and
evicts the entire journal's temporary storage. Data refusal does not become a
failed owner, and an explicit logical higher-epoch transition does not refresh
an original signed guard.

The three candidate tests observed:

- 80 state/error/schedule cases across 1/2/16/64/128 generations;
- 3072 initial leaf masks and 96 initial directory/inventory faults, with complete
  event-prefix checks and IO error precedence;
- 24 publication-entry/before-append/after-append/post-append-read cuts;
- 24 direct external latches, followed by refusal with no intervening validation
  or later IO, while original token/byte fingerprints survive.

After-append failures retain the new logical history/state but permanently refuse
further work; this is not permission to replay an uncertain attempt. Policy/key/
host admission, canonical Record validation, actual create/write/fsync, inventory
traversal, durability and process/SSH/controller loss are not modeled here.

## Test selection failure and repair

Reference selection had two arguments and remained an array. Candidate selection
had one, so the PowerShell if expression returned System.String. Splatting that
scalar into a native executable passed individual characters as separate filters.
The test harness consequently selected all twelve names, including old reference,
allocation and counter controls. Both broad-run receipts are retained exactly.
Old immutable evidence was not overwritten, but the intended no-old-cohort scope
was violated. The repeated allocation values do not become new evidence.

The repair returns an explicitly typed string array through a shared selector and
checks both filter identity and cardinality before any model launch. A standalone
native argv echo demonstrates the old character sequence and the repaired single
candidate argument/reference-plus-exact pair. It executes no model or Cargo test
image. Raw argv receipts, source snapshots and the first broad run are separate.
The corrected candidate-only execution has deliberately not been attempted.

The offline reader requires the actual twelve-test membership and unwanted old
row counts; changing the summary to three tests or dropping those rows refuses
review. It retains the red isolation decision even when scoped model outcomes
and argv repair checks pass. Old allocation values are ignored in derived facts
while their raw logs remain protected by receipt hashes and inventory SHA-256.
Diagnostic FNV checksums are not cryptographic seals or full engine identity.

## Verification and next gate

Three source guards join the previous 55, preserving frozen reference/storage,
actual change-source order and the absence of OS/clock/transport/launch behavior
in the image. Eight new offline reader controls join the previous 81. Twenty-one
older seals and 1183 members retain all previous first results, original windows
and refusals. Local format/check/clippy, docs/registry/performance checks do not
override the red target-isolation gate. No new performance or release claim.

Next run one explicitly identified candidate-only correctness cohort after
verifying the repaired native argv, with all legacy/allocation controls filtered.
Keep this broad run as negative selection evidence; do not replace or aggregate
it. After that gate, audit the exact actual-engine storage patch and its source
observer/snapshot boundary before runtime mutation. Real publication/fault,
all-writer termination, full SIGKILL/lost-ACK, reconnect and server admission
remain separate prerequisites.

Runtime, qualification and 300/60/10 windows are unchanged. No WSL, server,
clock/service change, installation, supervisor stop or product workload occurred.
Source history: plan `19f6c71d`, legacy `49f10265`, candidate `e7304785`, native
argv repair `302505d7`, offline reader `d4185616`.

Inventory SHA-256:
`e596bab55acdaa07a77410b41a7fe6e4633dedd76a167e06db6de4db0d8148f4`.
It covers 99 members, excluding inventory.json itself. All 20 local evidence/
source checks and their log hashes passed, including 89 offline controls and
58 source guards; raw working/staged bytes matched. This does not override the
separately refused target-isolation gate.
