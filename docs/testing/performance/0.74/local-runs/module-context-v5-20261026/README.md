# Contextual module closure evidence

This packet retains the first old-scanner refusal and the separate contextual
source-only capture on `6897daaed9aef668f86d77e3ce5076346c20a8f3`. Final closure:
365 Rust sources, 25,774 syntax sites, zero unresolved edges. Relative to v4:
21 changed rows, 21 added rows, no removals; these are source/AST differences,
not 42 product changes.

`index.json` SHA256:
`be8eb6bc81a7ddfb2dc118827f2946ebab0850e9d6aff1171657898e9a795dc0`.
Its 103 members reference 88 raw objects. Unchanged bodies are inherited only
from the separately pinned v4 packet; all 42 delta bodies are retained here.
No executable or private key is included.

One bounded invocation passes 23 controls: eight new resolver, three unchanged
old resolver, four new capture and eight historical v4-review cases. The compiler
control emits metadata for two synthetic libraries without running them.
All-target xtask check, strict all-target/all-feature Clippy and scoped formatting
pass. Captured commands retain the 1,200-second bound and four-GiB disk floor;
the syntax child retains its 120-second bound and eight-round closure limit.

The first added compiler control had two row-reordering assertions in the wrong
function: seven passes and one `NameError`. `derived/first-control-error.json` is
explicitly a manual transcript of the tool result, **not raw streams**. Moving
those statements back precedes the final cohort. The first offline verifier
then computed the repository root one parent too high and raised
`FileNotFoundError` for the old index; no integrity predicate passed or was
removed. The corrected root has a regression case. That first tool response
was not captured to a raw file.

Offline replay and seven integrity controls pass:

```text
python docs/testing/performance/0.74/local-runs/module-context-v5-20261026/verify.py
python docs/testing/performance/0.74/local-runs/module-context-v5-20261026/test_verify.py -v
```

Replay checks fixed packet identities, raw members, unchanged source receipts,
the complete 365-file body binding, exact delta and closure. It imports only the
hash-checked local Python resolver. No retained Rust, Git, Cargo, compiler or
fixture entry point is executed by replay.

This is **not** effect clearance, a package run, sandbox proof, runtime validation,
GNU/all-feature admission or qualification. A separate manual source/effect
review and auxiliary-input inventory must precede a new full
780-second/four-thread package command. V4 pins and its red result remain.
