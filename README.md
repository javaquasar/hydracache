# HydraCache 0.71 AX42 memory evidence

This orphan branch contains the D0 baseline and publication-safe copies of the D4 memory
campaign evidence. It is not a source-code branch and must not be merged into `main`.

The complete D0 baseline is in [`d0-baseline/`](d0-baseline/README.md): eight accepted M0-M7
campaigns, 105 successful jobs, 16 Git LFS archives, the extracted GitHub artifacts, an
immutable manifest, checksums, and `verify.ps1`. D0 was measured on source
`906aa24cc22ad6b50b824120ed6364208484203a` with workflow
`9c533d1de5a25b83bcb294e5b939064737b7d9fc`. This is a copy of the D0 material already
published in `main`; adding it here does not remove the earlier Git history or LFS objects.

D4 was measured on source and workflow SHA
`da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8`.

Each `campaigns/<immutable-id>/` directory holds a sanitized campaign archive and the
matching extracted GitHub Actions artifact. M3, M8, M9, and M10 were successful. The M10 B1 and C
jobs each completed their 24-hour measurement window. `manifest.json` binds campaign IDs to
GitHub run and artifact IDs. `SHA256SUMS` and `verify.py` verify the published archives and inspect
their members.

These are **sanitized derivatives**, not byte-identical raw exports. Test-generated HC2 private
keys/certificates and compiled build outputs were omitted; loopback addresses in 22 reports were
replaced; three
identity-bearing frozen-host files were omitted. The original campaign, protected-mirror, and
host-state archives are retained unchanged outside both AX42 and this Git repository. Their
hashes and the exact exclusions are recorded in `redaction-receipt.json`. The large raw
`host-state-and-admission.tar.gz` also contained the trusted harness checkout and `.env`, so it
is deliberately not in Git. The protected mirror archives contain nested job archives and are
also retained only in the external raw copy.

These measurements support only exact-host, exact-source, exact-workflow D4 memory claims. They
do not imply cross-host portability or a production-service-level claim. Git LFS is required for
materializing the compressed archives.
