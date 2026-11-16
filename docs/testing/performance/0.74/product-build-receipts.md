# Product build receipts for I74 and C74

The workflow builds the reviewed I74 and C74 commits with Rust 1.94.0
and the pinned Cargo lockfile for x86-64 Linux. It verifies each commit
and tree before and after the build. The signing jobs download the two
binaries from that workflow attempt and verify their hashes and role
identities before creating GitHub artifact attestations. Each attestation
binds its ELF and build receipt to the protected workflow.

The build commits and reviewed workflow must be available to GitHub.
The workflow runs manually from protected `main`. The
`performance-product-build-074` environment requires explicit deployment
approval from the sole reviewer `javaquasar` (GitHub user ID `22050429`).
For 0.74, the environment may allow that maintainer to dispatch the workflow
and approve the signing jobs from the same account. The deployment policy
permits only `main` and disables administrator bypass. The workflow checks
these protections before building and again after approval.

Download each signed artifact from the exact approved run. Check the
binary, signed predicate and signer against the reviewed workflow commit
and run identity before staging it on AX42.

```text
python scripts/perf/product_build_receipt_074.py verify-signed I74 ARTIFACT_DIR WORKFLOW_COMMIT RUN_ID RUN_ATTEMPT
python scripts/perf/product_build_receipt_074.py verify-signed C74 ARTIFACT_DIR WORKFLOW_COMMIT RUN_ID RUN_ATTEMPT
```

These receipts establish build provenance. The release admission also
requires the registered performance, host, compatibility and
qualification checks.
