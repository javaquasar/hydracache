# Local milestone, disk recovery and the dependency-policy frontier

The first complete `cargo xtask verify` attempt is retained as failed at
dependency policy, not a completed green milestone. Independently, the ordinary
Windows supervisor command passes all 350 cases across 86 targets, zero
failed/ignored/filtered; check and strict all-feature Clippy also pass. Windows
does not execute Linux-only fixtures or certify their clocks/manager path.

The [179-member packet](local-runs/local-milestone-20261024/index.json) has 119
objects and index SHA256
`77fbc293a2f9dc4c10cea302375af693f9331fea422b4b10e39350d3aa785e8a`.
It retains commands, first raw logs/results, source maps, exact cleanup lists,
public advisory records, independent resolver previews and package ownership
trees. Its protected identities are rechecked after cleanup. No ELF is added.

## Actual cleanup and recovery

Nonrecursive cleanup removed 986 individually verified old `.pdb` files in
`target/debug/deps`, with the final cutoff fixed at 2026-10-20 00:00 UTC. Targets
must be ordinary files in that exact directory, have the selected size/write
time and no reparse attribute. The command checks for active matching linkers
before deleting. Every selected path is recorded before the operation; all 986
are removed and none skipped. EXE, source, evidence and recent symbols are not
deletion targets. Existing qualification/other worktrees are not removed.

The deleted files total 27,100,012,544 logical bytes, about 25.239 GiB. Free space
observed around that operation rises from 20,548,456,448 to 31,191,715,840 bytes,
about 9.912 GiB. They were already compressed; logical size is not physical disk
recovery. Background work can affect free-space counters. Deleted copies are
not in the recycle bin and their exact old bytes are not recoverable here;
debugging symbols can be obtained again by rebuilding the corresponding target.

Earlier exact incremental-cache and PDB compression completed successfully and
is reversible. No incremental files were deleted after that deletion was
refused. Do not add logical deletion size to earlier compression savings or
claim their overlapping PDB stages freed that total twice. Later builds consume
some recovered space. Source and pinned images remain unchanged; the old native
image still hashes ca5484aa... and the protected GNU/MSRV parser images still
hash ae77d1bb... / ae33035a.... The complete values are in the packet.

## Full fast milestone: first outcome

Command: standard `cargo xtask verify`, source `7e6c8ee8a63ad8596a73383686444b54da04b863`.
The wrapper allows at most 4800 seconds, reserves 4 GiB and can terminate only
its own process tree. Neither limit fires: the standard gate exits 1 itself.
It is not a product qualification or a disguised six-hour performance run.

Passed before the stop: doc-check, management-center, local 0.73 performance
contract, release feature leak, full format, workspace all-feature/default
clippy, HydraCache common/system/mimalloc clippy and dependency bans. Dependency
policy reports fifteen advisories with bans/licenses/sources OK. Later full
gates, including its workspace tests, do not execute. No second full attempt is
made to obtain green. Separate earlier suites retain their own identities.

The recorded source-map and commit are equal before/after. The map covers its
explicit tracked source/config prefixes; it is not claimed to be an exhaustive
byte inventory of every workspace input. The tracked tree was also clean, and
no product crate changed in this session relative to d9a2b85c; changed crate
sources belong only to xtask. Cargo.lock, deny policy and qualification remain.

## Dependency audit is not an implemented fix

The [security work plan](dependency-security-plan-20261024.md) separates compatible
patch previews from major API/parser/build-chain decisions. Public registry dry
runs resolve five proposed patches after qualifying the roots package; no
manifest/lock bytes change and no proposed dependency is compiled. Offline cache
misses, ambiguous package selection and incompatible major constraints remain
first outcomes. A separate first audit-launcher UTF-8 decoding failure is also
labelled honestly; its raw stderr was not captured to a file.

Zbus 6's observed Rust 1.89 requirement exceeds the retained 1.88 contract.
Maintained-source/MSRV, YAML-governance and ZIP/Swagger migration need explicit
decisions. No ignore, target narrowing, package renaming or compiler override is
used to bypass the policy. The advisory database's clean postflight SHA is
84d943a6deeb8cc6c6f632531aaf4f1c62275239; a preflight DB pin was not recorded.

## Offline count consistency, not a new timing claim

The earlier native profile's outputs phase records 76544 Origin, 153088 Boot,
306176 Namespace, 19872 Sender, 39744 Alive/Projection and 232576 Document spans.
Six aggregate count relations agree with the unchanged code's complete paths.
The static Origin model has 56 kernel-facing operations per complete invocation,
or a nominal 4,286,464 at that count. This is a source-level model, not strace or
a measured syscall count; it does not prove per-event parentage or future cost.

Projection's 138900023 ns aggregate is about 1.94 percent of the old outputs
wall observation. This is not current CPU share, saved preparation time or a
prediction after the parser edit. Nested spans overlap and must not be added.
The synthetic allocation counts are not scaled into this native run. These
facts prioritize investigation of validation composition while its observation-
frequency change still needs approval; they do not authorize dropping checks.

## Closed admission remains closed

The separate read-only `performance-contract-check --release 0.74 --require-ship`
returns exactly the expected closed-admission refusal. The CLI image hash is
unchanged. This confirms a guard, not a successful ship gate. Local default
contract checks remain non-promotable, and qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

No release install, supervisor maintenance, clock/service modification, product
workload or expensive qualification is opened. The earlier separately scoped
native cost session remains its own failed diagnostic; no new server action
follows it. See the [closure audit](release-closure-audit-20261024.md) for original
execution, all writers, completion/later cells, reconnect and trust boundaries.
