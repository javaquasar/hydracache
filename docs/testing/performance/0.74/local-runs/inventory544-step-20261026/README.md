# Inventory projection repair results and first refusals

Index SHA256:
`b9aa94f2438307b77e672f966790da3704e4e5e6b341ce73b11ebc0d9dd4f013`.
The 55 logical members retain the first one-case RED, first eight-plus-five
candidate pass, first changed-source artifact integration (7 passed/13 failed),
final nine-plus-five cases on current/MSRV, six provenance controls, check,
strict Clippy, scoped format, documentation and local contract checks.

Every source-check wrapper records unchanged source/HEAD around its command.
The original RED and first-candidate source variants are retained and verified
against their original raw source hashes; reconstructing those variants is an
offline evidence operation, not another run. The final helper and tests are
also retained. Only source-only checks run: no server, service/clock operation,
original-owner, product measurement, qualification or full-package repeat.

The stale v4 review writes an explicit non-dispatching refusal. Old pins and
the first wider package remain unchanged. Keep `objects/* -text -diff` for raw
streams/source bytes. See the [repair report](../../inventory544-phase-result-20261026.md)
for the remaining body/COMPAT integration boundary.
