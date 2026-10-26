# Security successor input audit

This read-only audit separates the three approved locked security updates from
other source and effect drift since the reviewed V6 package. It does not project
historical inputs, update preservation guards, execute the package or admit a
release. It has no execute, test-filter or timeout override option.

Run with Python 3.11 or later, using the source-only syntax document retained in
the [result packet](../../docs/testing/performance/0.74/security-successor-audit-result-20261026.md):

```text
python tools/security-successor-audit-074/audit.py PATH_TO_SYNTAX_JSON
python -m unittest discover -s tools/security-successor-audit-074 -p test_audit.py -v
```

The audit reads current files freshly, checks six complete canonical bodies
against independent frozen Git inputs, and permits only CRLF-to-LF conversion.
It compares the complete TOML graph after exactly three version/checksum updates
and two declared-floor changes. Features, MSRV, roots, other nodes and dependency
edges must remain identical. Git inputs have a fixed revision/path allowlist;
this is an offline diagnostic, not a runtime source reader.

The old sealed V6 receipt is immutable data. A fresh syntax/source comparison
enumerates changed, added and missing rows and default targets; it does not reuse
V6's effect approval. Two explicit heartbeat companion inputs are separately
pinned because the Linux companion is absent from the old auxiliary catalog.
This is not a transitive Python review or compiler/macro expansion. Original
package wrappers retain their exact pins, 780-second bound and four threads.

Historical lock/source guard compatibility, current transitive effect review,
full package coverage and release admission remain separate gates. Old failures
and the frozen qualification manifest are not rewritten by this tool.
