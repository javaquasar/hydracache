# Historical Rust IO and independent current source gate

Thirty historical source-canary binaries now read their source inputs from the
fixed baseline commit instead of treating the current runtime as historical
evidence. A separate current-tree gate rejects unapproved current changes. The
owned-storage artifact remains unapplied, and its existing source guard is not
rewritten to accept runtime mutation.

Method commit: `fe5ebda429d9ef1f3a9ccd059600d264936cc283`.
Historical source: `33c54b0cda8c330b115009cdf092ad002077cedf`.
See the [preregistered plan](authority-rust-history-plan-20261022.md),
[IO adapter](../../../../crates/xtask/tests/support/authority_history_io.rs),
[complete-file contract](../../../../tools/authority-rust-history-074/contract.rs)
and [independent tests](../../../../crates/xtask/tests/authority_rust_history_074.rs).

## Historical reads and Git differences need the same boundary

The earlier integration audit named fourteen guards directly tied to authority
sources. The full cohort also contains guards that compare the whole src/tests
tree. Redirecting only fixture reads would leave those differences coupled to
the current checkout. Thirty binaries therefore receive the same explicit IO
adapter; none of their test bodies or assertions are removed or ignored.

The allowlist replaces 32 actual read calls and 40 Git command constructions,
plus one module declaration per file. Complete token comparison against each
frozen original proves that no other code changes. An independent AST control
counts real calls, including a Git call inside an assert_eq block expression;
literal contents are not interpreted as calls. All 79 original test names remain.

Reads return Git blob content from the exact historical commit. Unsupported or
missing paths fail without falling back to the working tree. Original show and
ls-tree revision operands stay intact. For diff, the adapter inserts only the
fixed historical right-hand revision before the original path separator, keeping
the original flags and every path. It does not filter unexpected files out of
the result. Git object replacement is disabled. Unsupported commands, argument
shapes, mutable HEAD revisions, alternate cwd and unsafe path names refuse.

Normalizer support files remain compilation inputs in the current repository;
the independent gate requires their existing sources to remain unchanged. This
is a source-test IO adapter, not an OS sandbox or a portable standalone bundle.
Reproduction requires the repository and the pinned Git history.

## Current drift has its own gate

The new current-tree test examines Git-reported changes to pre-existing tools
and crates files, manifests, contract TOML and qualification. It permits only
the exact thirty IO substitutions. Other existing source changes refuse, as do
new tracked or untracked runtime src/tests files. Whole-file checks inspect all
thirty migrated guards independently of that changed-file list.

Runtime fixture and observer are still baseline-only. Negative controls present
the valid owned-storage artifact as a current runtime change and require refusal
while historical reads remain fixed. Other mutations target an unrelated model,
normalizer support, added or ignored tests, assertion replacement, IO bypass,
extra helpers, duplicate modules and unsupported command forms.

The prior four candidate/observer/contract/test sources remain pinned separately
to their committed method. Qualification has an additional raw SHA-256 check,
not only a text or Git-diff comparison. Historical success cannot substitute for
those current checks. This closes the Rust source-routing prerequisite only;
it does not authorize applying the storage candidate automatically.

## Python history stays immutable

The frozen Python reader view still reproduces the archived integration facts.
Its source pins, reader predicates and sealed packet bytes are unchanged. Old
raw audit commands intentionally pin the original guard files; after this IO
migration they must not be run against the adapted checkout and described as
historical replay. Use the generated historical view for that purpose.

This cohort runs the twelve view and eight receipt controls. It does not rerun
or claim a new pass of the old complete 129-control raw Python cohort. The
previous sealed packet remains evidence of that earlier result. No measured
Rust model image runs in either historical source-check route.

## Development refusals remain separate

The first development cargo log records six of eight new controls passing and
two refusing module placement/order. The preceding rustfmt also reported an
inner doc comment after the new module declaration; that diagnostic was in chat,
not in the redirected cargo log. The repair places the module after leading
inner documentation or after the existing guard declaration without moving the
original modules or assertions.

An additional pre-freeze control then refused because syn's default visitor did
not inspect macro argument token streams. A real Git call inside assert_eq was
missing from its count. The repair visits successfully parsed expression
arguments, not literal contents or arbitrary macro expansion. The first partial
canary cohort remains distinct from the later complete source-canary pass.

Raw development and first final logs, source snapshots and hashes are retained
in [the evidence packet](local-runs/authority-rust-history-20261022/). Development
commands preceded method freeze; their receipts do not claim a complete frozen
source image. The first final cohort uses the committed method and a no-clobber
output directory.

All fifteen first final steps passed on MSVC Rust 1.94: the 79 historical source
tests in thirty binaries, ten new separation/current-tree controls, eight prior
candidate-source controls and twenty Python history-view/receipt controls.
Formatting, package check/strict clippy, documentation, the registry check for
186 discovered gates and the local non-promotable performance contract passed.
The documentation follow-up also passed. This is not a full release verify or a
claim that all 186 registered gates were executed.

The preceding storage-source inventory and all forty members remain unchanged.
New evidence commit: `ef0cd964a0a8f0848187a629b16f4669a61b77f5`.
The new inventory covers 61 members, excluding inventory.json, with SHA-256
`4904ede1ef28f2e4d96b1da0f5be8b633c20a3d2fbb6524289567b974581b63f`.
All 62 packet files have identical raw working and staged bytes.

## Next integration boundary

Apply the minimal runtime patch only with a separately versioned current-runtime
candidate gate, then Linux-typecheck mutable call sites and test actual
temporary-filesystem cache success, refusal eviction, reopen and publication.
Historical Rust IO no longer blocks that work, but it supplies no Linux compile
or real filesystem result itself. The old candidate-source guard still requires
an unchanged runtime until that explicit integration step.

No allocation, latency, layout, copied-byte or retained-memory gain is measured.
Original validation count/order, 300/60/10 windows and first Causal refusals
10042390247/10010835739 ns remain unchanged. Qualification remains exactly
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
No WSL, live/original-owner or real fault attempt, server, installation, supervisor,
product workload, clock or service action occurred. Publication/all-writer/full
SIGKILL/lost-ACK/reconnect/server admission gates remain separate.
