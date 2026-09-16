# Archived performance evidence

This directory contains immutable raw campaign copies retained for audit, offline analysis, and
comparison with future dedicated-host campaigns. Each campaign lives under its release and unique
campaign ID. Existing campaign files must never be edited or replaced; a later run must use a new
directory.

## Archive branches for new campaigns

Starting with the 0.71 D4 candidate, keep large raw archives and extracted GitHub artifacts on a
dedicated evidence branch, not on `main`. Use one branch per accepted release/host/candidate set:
`evidence/<release>/<host-alias>/<candidate>`, for example `evidence/0.71/ax42/d4`.
Create it as an orphan branch so its tree contains evidence only and does not inherit the source
tree. Within it, use `campaigns/<immutable-campaign-id>/` for each campaign, plus a top-level
`manifest.json`, `SHA256SUMS`, `README.md`, and a verifier. Never replace a campaign or reuse its ID;
append a new campaign directory for a corrected run. Track compressed archives with Git LFS.
The orphan branch must carry its own `.gitattributes` LFS rules. Scan every file and archive
member for secrets and host identifiers before the first push; sanitize sensitive members and
record redaction receipts without altering the original external copy.

Keep a small, reviewable index on `main` under this directory. The index must record the exact
evidence branch name and commit SHA, measured source/workflow SHA, campaign IDs, GitHub run and
artifact IDs, archive filenames/sizes/SHA-256, verifier result, and claim boundary. Link the
release report to that index. Do not merge an evidence branch into `main`; changing its tip requires
an index update, and accepted commits should remain reachable until retention is explicitly
reviewed. Verify the archive branch from a fresh checkout with LFS materialized before accepting
the index. Preserve a second verified copy outside both the rented host and this Git repository.

Deleting an evidence branch is **not** a guaranteed GitHub/Git LFS storage reclamation operation,
and it must never be the only remaining copy of accepted results. Before deletion, verify another
durable copy, retain its checksums and retrieval location in the index, and change the index to
mark the branch unavailable. Archive branches are publication and checkout isolation, not a
substitute for backup or retention controls. Existing 0.67.1 and 0.71 D0 evidence already merged
into `main` is grandfathered; do not rewrite its history merely to adopt this layout.

If a not-yet-merged release branch already contains new evidence, audit its commits and paths
against `main` first. Preserve its old tip under a temporary backup ref, copy and verify the
evidence in the orphan archive branch, then rebuild the release branch with code and documentation
only. Compare the resulting non-evidence tree and commit list with the original before replacing
the remote branch. Never force-rewrite `main`, a merged branch, or a branch with uncoordinated
contributors. Remove the backup ref only after the archive branch, release branch, and external
copy have all been verified.

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

## 0.71 AX42 D0 memory baseline

The complete eight-row D0 memory baseline is archived under
[`0.71/ax42/d0-baseline`](0.71/ax42/d0-baseline/). Compressed server and mirror copies use Git LFS;
the downloaded GitHub artifacts remain ordinary Git files so receipts and measurements can be
searched and reviewed without unpacking a campaign archive.
