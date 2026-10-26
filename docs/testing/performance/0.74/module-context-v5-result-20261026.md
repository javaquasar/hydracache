# Source-only package closure after the reader repairs

The current package module graph closes at 365 Rust sources and 25,774 syntax
sites. This completes inventory construction, **not** source/effect review or
the full package gate. V4 pins and its first red result remain unchanged. No
server, runtime, dependency, live-check schedule, deadline or threshold changes.

Implementation commit: `3a86b1c6ba603446b328cc7fc20778c63554e5a3`.

## First refusal belongs to the inventory tool

After the [reader repair](remaining-readers-result-20261026.md), the bounded
metadata build of `census_cost_074` passes on clean source
`6897daaed9aef668f86d77e3ce5076346c20a8f3`. The syntax child parses 361 sources.
The old scanner then refuses two edges from
`tools/current-frontier-074/reader.rs`, looking for children under `reader/`.
Its complete streams, edges, refusal and unchanged-source receipts remain in the
[packet](local-runs/module-context-v5-20261026/README.md).

The reader is declared through `#[path]`: its default children resolve relative
to the containing directory, unlike an ordinary `reader.rs` module. The real
compiler already accepted this graph. The scanner did not model the distinction;
no missing file is ignored to clear its refusal.

## Separate declaration-context resolver

`tools/historical-blob-batch-074/module_context_v5.py` discovers root, explicit-path
and ordinary declaration contexts to a finite fixed point. Every module is
resolved in every discovered context. Missing, ambiguous, conflicting,
outside-root, duplicate and unowned declarations refuse; it never chooses the
first directory containing a candidate. Bodies remain hash-bound.

`syntax_audit_v5.py` uses that resolver inside the unchanged capture algorithm
and restores the old resolver afterward, including failures. The old scanner
must match its fixed SHA256. Its eight-round limit, fixed syntax-only child,
build/source receipts, 120-second child bound and final body checks remain.
Capture neither executes input Rust nor expands its macros. This is not a
sandbox or whole-program reachability proof.

Contextual capture adds four real files and closes on round two. Against the old
344-source document: 21 changed and 21 added rows, no removals. Those 42 bodies
are retained separately; 323 exact unchanged bodies come from the pinned old
packet during offline replay. A read-only comparison also finds all 29 old
auxiliary pins unchanged. Additional reader descriptors and effect implications
have **not** yet received a new complete admission review.

## Checks and first results

Twenty-three controls pass together: eight resolver, three unchanged old resolver,
four capture and eight unchanged historical v4-review cases. Two synthetic Rust
libraries compile to metadata to distinguish explicit-path and ordinary lookup;
neither runs. All-target xtask check, strict all-target/all-feature Clippy and
scoped formatting pass. Offline replay and seven integrity controls also pass.
No original-owner, live fault or measurement cohort starts.
Final documentation and local, non-promotable performance-contract checks pass.

The first compiler test itself has a `NameError` after two assertions were placed
in the wrong function. Its manually transcribed result remains labeled as such,
without claiming retained raw streams. The first offline replay also refuses
because its root constant is one parent too high. Correcting that root leaves
the integrity predicates unchanged and adds a regression case. The packet README
records both limitations. No cost or throughput comparison is claimed.

## Remaining admission work

Next: manually review all 42 source/AST deltas, process/filesystem effects, newly
included inert data and environment/build inputs. That review requires its own
fixed v5 identity and refusal tests; current capture output alone supplies no
admission pin. V4 must keep rejecting this changed graph.

Only then can a complete default-Windows package use the original
780-second/four-thread command. Focused passes cannot substitute for the
incomplete main target or unstarted suffix. Dependency decisions, C74 scope,
production original delivery, all-writer retirement, lifecycle, reconnection and
real enrollment remain separate open gates.
