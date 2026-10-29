# Remaining approved Zstd fault gate

Baseline: clean `27d51d666979b56d4790d210a404c05384abf281`; the
[first changed-candidate workspace milestone](windows-markdown-workspace-result-20261029.md)
refuses RUSTSEC-2026-0391/0398/0403. Root lock SHA-256 is
`34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c`.
No identical whole-workspace rerun is scheduled.

The already reviewed dependency candidate has only partial security controls:
seekable and corrupt legacy streaming behavior were explicitly unqualified.
Complete those controls with the
[fixed local method](../../../../tools/zstd-security-fault-074/README.md), using
official archive identities and selected-feature upstream tests. Preserve the
first context baseline refusal and both MTU/allocation declines. The old known
unsafe paths need not be executed to test the patched contract.

Do this before a distribution/adoption decision. Retain exact sources, locks,
artifact/listings, first raw streams and full case accounting; qualify only the
tested APIs/features/platforms. No new performance hypothesis, root dependency,
deadline, exception, server or service change is part of this step.
