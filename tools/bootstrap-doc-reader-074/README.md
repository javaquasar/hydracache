# Bootstrap document reader phase

This test-only adapter reconstructs exactly the previously reviewed COMPAT and
gated-test-registry document phase for nine bootstrap source-contract readers.
Every other read retains its bytes, order and I/O error. Each call performs one
fresh read; only fixed expected descriptions are cached. Lexical path selection
is not filesystem authorization or a claim of inode/symlink atomicity.

The document projection reuses the independently pinned source-successor rules;
it does not accept hashes learned from the working tree. Whole current and
predecessor document hashes and every inverse edit are checked. Unknown document
changes, stale predecessors, partial edits and duplicates refuse. Existing
bootstrap assertions and historical normalization rules remain unchanged.

`rules.json` describes exactly eleven caller edits from commit
`181c077abdfbae5a8c4dfe67cee6f1c95519d529`: nine bootstrap readers (17 read sites),
the preceding installed-reader proof, and its authority-phase proof reader.
`contract.rs` checks complete predecessor and successor bodies. `reader.rs` and
`proof.rs` extend those earlier proofs without removing their assertions.

Validate the fixed description with `python tools/bootstrap-doc-reader-074/generate.py`.
`--write` creates it exclusively from the fixed predecessor; `--patch` emits a
patch only while every caller still equals that predecessor. Neither mode
approves an arbitrary current checkout. The tests are in
`crates/xtask/tests/bootstrap_doc_reader_074.rs`.

No production path, dependency, live predicate, observation frequency, admission
deadline or qualification threshold changes. This is not full package acceptance:
the complete current-tree audit still refuses inputs outside this narrow phase.
