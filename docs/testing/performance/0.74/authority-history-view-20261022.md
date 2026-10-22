# Isolated historical authority replay results

The new historical view replays the unchanged authority audit from exact commit
33c54b0c and reproduces its archived facts. It reads a separately materialized
source/evidence tree, not the current runtime as a fallback. This closes the
offline Python reader-view prerequisite only. Existing Rust guards remain intact;
the owned-storage candidate observer and semantic guard are not implemented.

See the [preregistered plan](authority-history-view-plan-20261022.md),
[text image amendment](authority-history-view-text-image-amendment-20261022.md)
and [retained facts](local-runs/authority-history-view-20261022/facts.json).
Formal replay source: `632cccb5`; input-manifest identity starts `8959f5c7`.

## Historical replay and current source are separate decisions

| Check | Input | Permitted conclusion |
| --- | --- | --- |
| Historical replay | 129 pinned sources and 24 sealed packets from 33c54b0c | Old facts reproduce unchanged. |
| Current baseline byte identity | Actual current fixture source | It still matches the original baseline bytes. |
| Future candidate semantic guard | Not implemented | No runtime candidate admission. |

A temporary mutated current-source copy fails the independent baseline guard
while the historical view remains valid. That is the intended separation, not
permission to accept the mutation. No existing guard is redirected to history,
and no current candidate is inferred from a green archived result.

## Bounded materialization and reader execution

The helper requests allowlisted paths from the fixed Git commit. It rejects
unknown/missing/duplicate/case-aliased entries, path traversal, device names,
links/special files, incorrect hashes and frozen size/member bounds before
importing historical code. The complete view contains 1576 files: 129 sources,
1423 sealed packet members and 24 inventories. No old measurement image executes.

A fresh isolated Python process imports eighteen unchanged reader modules from
that view. Their ROOT and all data reads stay separate from the working tree.
The diagnostic Path.open guard denies outside-view reads and writes during
analysis; it is not an OS sandbox for arbitrary Python. Every source and evidence
member is checked again after analysis. Only the validated unique target/ahv-
temporary directory is removed. The formal receipt retains raw worker stdout/
stderr, their hashes, the tooling commit, archive digest and retirement result.

## Preserve exact source bytes as well as semantics

The first unit setup refused with Qualification drift before executing controls.
Raw inspection found 2979 LF bytes in the qualification Git blob/worktree but
3068 CRLF bytes in git archive output. The initial explanation reversed that
direction; the separate amendment corrects it from byte evidence. Initial source
and the refusal description remain retained. No raw first unit log was captured;
later successful logs are not relabelled as that first output.

All 129 frozen source images are UTF-8 LF without BOM. The adapter verifies
canonical archived source content, reconstructs that explicit representation and
checks the same pinned raw digest before import. Sealed evidence bytes never
pass through the adapter. Qualification remains exactly
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc;
no expected digest, reader predicate or workload threshold is changed to pass.

The repaired development cohort passed eleven controls. The final source adds
raw worker-output capture and an explicit qualification-byte regression; all
twelve view controls pass. The formal first replay then reproduces the old audit,
including the clean targeted model gate and separately refused broad cohort.
Receipt mutation tests run without replaying the historical worker.

## Remaining implementation boundary

Next implement the exact current owned-storage guard and versioned observer,
covering fresh read storage substitution, mutable receivers and all five journal
cleanup paths. Historical Rust normalization/source guards still require explicit
historical versus current-candidate wiring; this Python view does not waive them.
Only then apply the minimal runtime patch and check actual filesystem behavior.

There is no new allocation, timing, layout, copy, RSS or product improvement
measurement. Runtime, qualification, original 300/60/10 windows and first Causal
refusals 10042390247/10010835739 ns remain unchanged. No WSL, live/original-owner
profile, real fault attempt, clock/service change, server/install/supervisor or
product action occurred. Real publication/all-writer/full SIGKILL/lost-ACK/
reconnect/server admission gates remain separate.

All 25 first local check steps passed: 129 offline controls, 79 source guards in
30 binaries, format/check/strict clippy, documentation, 186 registered gates and
the local non-promotable performance contract. Those source guards still check
the unchanged current runtime; none were silently redirected to the historical
view. The first unit setup refusal remains a separate development result.

Inventory SHA-256:
`fb3a40ab880b2a1d9129adc425670b21bad355ae07f66ea2eec6b45a14cf3afc`.
It covers 55 members, excluding inventory.json. All 56 packet files retained
identical raw working/staged bytes. Evidence commit: `7e786f9d`. Completed
documentation checks have a separate receipt inside the packet.
