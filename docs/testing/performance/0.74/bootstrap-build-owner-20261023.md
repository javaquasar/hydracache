# Signed bootstrap provenance bound to the consumed material owner

The bootstrap now has separate signed policy and build receipt formats plus a
compiled join that consumes the original MaterializedHeldUnit. The join retains
its protected input and a sealed bootstrap image under that owner. It exports no
execution, transport, release, completion or retry capability. Local static and
filesystem tests and deterministic owner models are distinct from the original
live join, child entry and server admission, which remain closed.

## Independent policy and build signatures

The [plan](bootstrap-build-owner-plan-20261023.md) starts from dd820049. The fixed
enrollment file selects the independent bootstrap builder key, complete signed
policy digest and bootstrap image digest. Policy verification does not derive a
trusted key from the incoming policy or receipt. Existing root/ancestor/file
checks and four-way old-role key separation still bracket the new verification.

Policy and receipt each have their own Ed25519 signature domain and canonical
newline JSON schema registered in COMPAT. Their signatures cannot be substituted
for each other or the old held-build signature. The policy binds repository,
builder/controller keys, authority epoch, exact source commit/tree/root lock,
bootstrap image, original held policy/receipt and host/launcher image digests.
The receipt binds that complete policy, epoch, fixed build observations, image,
lock and log. Unknown/duplicate/missing/noncanonical fields, future versions,
wrong domains/keys/pins/epoch and zero or over-budget digests refuse.

sign_policy validates the fixed purpose and supplied independently selected key;
sign_build derives its body only after checking observation and content. The
fixed command selects GNU 1.94, release, no default features, x86_64 GNU and the
diagnostic_held_bootstrap target. A bounded Cargo projection requires the exact
package/source/output/profile/features and a successful final event, retaining
the entire log digest. These signatures are trusted build assertions, not
independent proof a clean isolated build actually occurred. Tests use synthetic
keys, minimal ELF fixtures and synthetic Cargo events. No production key,
installed configuration, real bootstrap executable or production receipt is
created in this step.

## Original ownership remains linear

join_bootstrap consumes the existing material owner rather than borrowing a
caller-selected replacement or extracting an earlier context. Its new wrapper
retains policy/receipt bytes, the protected file's original identity and bytes,
and one readonly, CLOEXEC, exactly four-sealed bootstrap image. Fresh positional
ELF/hash/metadata checks compare the same object, not a reopened pathname or
copied launcher inode. A same-byte protected-file or image replacement refuses.

Every wrapper round runs the original complete material frame, checks the new
source/signatures/original digests/image, runs another complete frame, repeats
the new checks and closes with the complete frame after hashing. The original
held policy/receipt and host/launcher digests come only from that consumed owner's
Context. The retained observation also compares the complete Context, including
state, origins/bounds, owner, roots, stream/descriptor inventory, guardians and
manager names/serial. No cached success or new clock origin replaces a check.
All preceding validation bodies and absolute 300/60/10 windows remain unchanged;
the three additive module hooks are tested by reconstructing entire old files.

Any new error calls retirement on that same material owner and latches its held
trust refusal. Failed construction retains the refused wrapper/owner. Explicit
Drop retirement and owner-first field order precede closing the added image FD.
This delegates to the existing retirement contract; model refusal does not prove
cessation of every real writer. The added FD is private to this nonexecuting
wrapper. Its eventual transport/inventory placement still needs its own join.

Runtime policy and receipt copies are bounded to 4096 and 65536 bytes. Protected
input remains bounded to 2048 bytes. Image verification streams at most 128 MiB
through a 64 KiB buffer; it does not allocate or map the whole image. No source
or FD getter, owner recovery, start/send/release/unlock or fallback is added.
Dropping the new path does not make its consumed attempt available for retry.
Older binaries do not read the new formats; old artifact schemas are unchanged.

## Local checks and retained first outcomes

The integration baseline failed before implementation because the two new module
files did not exist. A subsequent current-source contract needed whitespace
normalization after rustfmt. Later source gates correctly noticed the added
material module hook in two earlier exact-file comparisons. Their adapters now
remove only that exact hook before comparing the entire preceding body; hook
duplication/mutation and unrelated body drift still refuse. First captured
outcomes and fixed-runner failures remain separate from final checks.

No original-owner live fixture, privileged namespace/mount, executable start,
service, clock, server, product workload or performance cohort is run. The
previous sealed-bind EINVAL and whole regular-file cleanup failure remain in
their original packet and are not reclassified. Compile/test elapsed time is
operational metadata, not a CPU, latency or allocation measurement.

Final fixed checks passed on stable source:

- Windows: 26 current/historical source-contract tests, five portable static
  build tests, all-target check, all-feature strict Clippy and formatting.
- GNU 1.94 and MSRV 1.88, each: six build/image tests, four protected selection
  tests, three owner fault models, five preceding enrollment tests and three old
  source/key/replacement regressions, for 21 selected tests per compiler;
  all-target check and all-feature strict Clippy. The preceding default-feature
  Reading::Reference test-support warning remains unsuppressed in Linux check.
- Three offline runner/retention controls, doc-check, gated-test-check with 186
  registered gates, local non-promotable performance-contract-check and diff check.

The first Linux lanes had passing commands but failed source stability while
source-contract adapters changed. They are retained as failed lanes, not admitted
as evidence for the final source. host-final/194-final/188-final are distinct
component checks, not retries of an original live or measurement attempt. Earlier
uncaptured process outputs are identified explicitly as captured-tool summaries,
not invented raw receipts.

The [exclusive packet](local-runs/bootstrap-build-owner-20261023/audit.json) checks
source commit `d53da397`; its [inventory](local-runs/bootstrap-build-owner-20261023/inventory.json)
has 201 members excluding the inventory itself, SHA-256
`c52ad71a1a813d1869ea18a56401777ecefb3469af5d0e79f1f3f65dec5009fc`.
The packet separates static/source/filesystem/model success from false original
live join, bootstrap execution, delivery, writer cessation and admission flags.
Plan commit is `685d1871`; fixed evidence tooling is `010c3d75`.
Evidence commit is `fe792f59`. Before commit, a raw staged-Git-blob audit verified
all 202 packet files including inventory against their retained sizes, digests
and worktree bytes; no line-ending conversion changed the sealed packet.

The frozen qualification SHA-256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

## Remaining execution boundary

Next implement the independently enrolled bootstrap executable and fixed child
entry: authenticate the admitted original context, exact received launcher FD,
environment/credentials and manager/guardian bindings, then one execveat without
pathname fallback. Extend the closed FD inventory before any delivery. Prove
original live join in the existing windows, then original delivery, all-writer
cessation and new SIGKILL/lost-ACK boundaries. Completion/cells 2–4, reconnect,
production enrollment and server rehearsal are not opened by this component.
