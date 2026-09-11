# Archived performance evidence

This directory contains immutable raw campaign copies retained for audit, offline analysis, and
comparison with future dedicated-host campaigns. Each campaign lives under its release and unique
campaign ID. Existing campaign files must never be edited or replaced; a later run must use a new
directory.

The repository is public. These archives may contain non-secret operational identifiers such as a
Linux boot ID, a root-filesystem UUID, synthetic test-node identities, internal container network
details, process IDs, timestamps, and detailed host configuration. The archived AX42 campaign was
scanned before publication: no passwords, private keys, GitHub token shapes, public host address,
physical NIC MAC address, disk serial number, disk WWN, or production/user payload was found.

## 0.67.1 AX42 reference campaign

- Campaign: `hc0671-ax42-20260911-da`
- Candidate commit: `7bd31af9a5092466d7a7284995f388d33ed3110f`
- GitHub Actions run: `34580969962`
- Original artifact ID: `10261605556`
- Original artifact SHA-256:
  `c5d4f18e037251ba66faac050460c47126d57cd42e916e2c9fa97d444df4ae2a`
- Anonymized report:
  [`ax42-reference-0.67.1-20260911.md`](../perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md)
- Raw campaign copy: [`hc0671-ax42-20260911-da`](0.67.1/hc0671-ax42-20260911-da/)

The original ZIP and host-state archives remain unchanged. Derived reports must identify their
source campaign and must not broaden its exact-host, exact-source, and same-box claim boundaries.
