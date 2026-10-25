# Historical reader input phases

These are source-test routing changes, not a runtime, enrollment or release gate.
The fixed predecessor is commit `628bc3c113ce12e9ecb10ee4daee6961a6b2ad15`.
`contract.rs` records exact whole-file predecessor hashes and constructs the
five declared reader edits. The companion tests compare those constructions to
the actual sources and preserve all eighteen original cases in those five files
and every assertion after reversing two explicit input-expression substitutions.
Four cases in the two untouched context/key-hex targets remain separate controls.

The registry readers first check the current body through the independently
pinned source successor, then exercise their unchanged old predicates and
comparisons. The historical input test keeps the original entry separate from
the verified predecessor. The authority-runtime test likewise uses a verified
predecessor for old body checks but supplies freshly read current fixture bytes
to the current `view` entry point. That gate must still refuse stale predecessor
bytes. The current-tree reader checks the exact extended routing, not a broader
file or assertion exemption. Neither runtime gate nor existing source-successor
rules are changed here.

`main.rs` handles one further independently reviewed source delta: the terminal
GET-owner rollback at `a67a5299ab335424e9b11330dc06bd749ea9b859`, relative to
`37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a`. It checks the complete current main
test-source SHA256, reverses five exact text edits and checks the whole predecessor
SHA256. The returned string is used only for the old authority source assertions.
It is never compiled, executed, written back or used to re-enable a selector.
Current product code, Cargo features, numerical thresholds and the actual main
performance-contract source stay unchanged.

The main edit description has SHA256
`9373e36370e3893316a983fb159debba29afbbf4095c86f8f71d4384c80cc4f5`.
`generate_main.py` reconstructs it using the existing fixed-commit producer,
without replacement objects. Its default mode compares existing data; `--write`
creates exclusively. No HEAD, current index or observed response supplies approval.
Only immutable expected data is cached, never a current source observation.

```text
python tools/authority-reader-phase-074/generate_main.py
cargo test --offline --locked -p xtask --test authority_reader_phase_074 --test authority_runtime_074 --test authority_hex_reader_074 --test historical_input_view_074 --test owned_case_074 --test current_tree_reader_074 --test authority_context_074 --test authority_key_hex_074 -- --test-threads=4
```

The six new cases verify exact reconstruction, original test/predicate
preservation, missing/duplicate/wrong-branch/assertion drift, fresh current versus
old fixture routing, all eighteen historical inputs and their current mutants,
and the one-file terminal source projection. These do not prove runtime semantic
equivalence, an atomic filesystem snapshot, a numerical gain or package/release
admission. The retained first failures remain separate evidence.
