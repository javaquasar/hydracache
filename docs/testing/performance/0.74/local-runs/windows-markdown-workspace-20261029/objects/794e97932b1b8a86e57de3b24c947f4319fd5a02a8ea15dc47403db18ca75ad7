# Bootstrap document reader phase

The original test-only adapter reconstructs exactly the previously reviewed COMPAT and
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

## Approved security inputs and exact assertion fixtures

The integrated readers now separately select exactly `Cargo.lock` and
`tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs`. Both pass through
the unchanged roots/security successor checks after the same single fresh read.
The standalone helper reuses its existing successor module; the installed helper
reuses `crate::runtime::successor`. No second module instance, Git read or filesystem
observation is added. The old two-document `selected` and `project` functions and
the observation/error primitive remain unchanged. Unknown paths keep their bytes.
The selectors remain lexical, not filesystem authorization.

`fixture.rs` is a separate entry for the old document-source test. Whole current
COMPAT at `d3256fc08b8da2df424d6da5bce99c0e168939b0` is checked before its exact
diagnostic prefix is removed from the assertion fixture. Only that complete
older positive fixture may be expanded back before testing the unchanged fresh
document projector; the actual reader still refuses that older document directly.
Its `read_to_string` remains an alias of the actual fresh reader. The fixture
callback preserves one observation and errors, without re-reading a file.

The same assertion entry checks the whole authority-reader source at
`5ec4dd647195c286299842c9faa71d3c2fa58efd` before reversing only its exact import
and terminal-bridge call. This restores the preceding source for the old eleven-
caller comparison, not the compiled current reader. All nine original bootstrap
callers, old rules, support guards and original assertion tokens remain exact.

The eleven-target cohort passes 67 cases on Windows 1.94, Windows MSRV 1.88 and
GNU/Linux 1.94. The first candidate's fixture callback and missed authority-source
boundary, plus the later strict-Clippy unused import, remain first outcomes.
After consuming that existing re-export in the new test, only the changed target
is rerun: eight cases per environment; all-targets check and strict Clippy pass.

```text
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --test bootstrap_build_owner_074 --test bootstrap_child_boundary_074 --test bootstrap_child_census_074 --test bootstrap_child_reader_074 --test bootstrap_current_image_074 --test bootstrap_doc_reader_074 --test bootstrap_entry_binding_074 --test bootstrap_image_census_074 --test bootstrap_installed_origin_074 --test bootstrap_enrollment_074 --test bootstrap_current_view_074 -- --test-threads=4
```

See the [result and preserved first outcomes](../../docs/testing/performance/0.74/bootstrap-current-view-result-20261027.md).
This focused source compatibility result does not close the old red V8, incomplete
Linux frontier, held-role/diagnostic boundaries, security or original execution.
No new performance hypothesis, server, service, clock or product workload is used.

The [held-source continuation](../../docs/testing/performance/0.74/held-current-view-result-20261027.md)
makes only the already imported successor module visible for reuse by activation's
test reader. Every facade function stays byte-identical; no new reader or
production authority is exported. Sharing its existing security module avoids
compiling a second instance. The 33-case held/bootstrap regression passes on
Windows/MSRV/GNU, separately from the diagnostic-artifact target's 20 cases.
