# Extended local readiness audit — 2026-10-11

This supplements, rather than edits, the
[44-capture signed mount packet](../diagnostic-signed-mount-20261011/README.md).
It contains 24 closed raw captures, their byte counts/SHA256, and a read-only
`verify-packet.py`. Neither packet is enrollment or release qualification.

Native Windows `cargo +1.94.0 run -p xtask --locked -- verify` exits 0.
Its 532 successful harness result rows total 3,982 passes and 82 existing ignored
executions. These are **not unique tests**: DST, soak, failpoint and budget tests
also run in other gates. Ignored external DB, scheduled/chaos and platform-gated
cases are not certified. Strict clippy/allocator lanes, dependency policy,
workspace/xtask tests, doctests and workspace rustdoc pass. Duplicate dependency
warnings remain visible.

The built-in console gate explicitly skips because native `npm` is not found.
Separate `npm.cmd` install/build/test runs pass 16 Vitest, six package and 46
browser tests, twice. The later test-only nested-copy follow-up passes 16 Vitest
and seven package tests. Browser fixtures are unchanged; it is not another
production or qualification run.

The original full npm audit reports two high-severity dev packages:
`brace-expansion@5.0.9` and `source-map-js@1.2.1`. The minimal lockfile patch
selects 5.0.12 and 1.2.2 within existing parent ranges, using npm registry
integrity metadata. Both lock regressions first fail, then pass; a later nested
copy/missing-package canary first exposes a false positive and passes after review.
The repeated full audit reports zero vulnerabilities at capture time, without
omitting dev dependencies. Production-only audit is separately retained. This
does not predict future advisory status or attest the final release.

Vite preserved Windows input HTML line endings. Pinning `console/index.html`
to LF and rebuilding removes the incidental generated/embedded Git content
changes; no generated asset blob is committed. Local same-host deterministic
bundle checks pass; cross-host package reproducibility remains unproved.
The 182-component console SBOM is local metadata for package version 0.73,
not the missing final 0.74 source-bound release SBOM.

Linux all-targets supervisor check additionally passes on MSRV Rust 1.88
(installed locally, without changing the default toolchain). Scoped Linux rustdoc
passes on 1.94 with `RUSTDOCFLAGS=-D warnings`. Windows MSRV evidence is in the
earlier signed mount packet.

The long Windows verifier started before the first implementation commit and
spanned subsequent reviewed source/docs/security changes. It is a working-tree
audit, **not an immutable candidate receipt** or registered
`fast.workspace-nextest` success. The 840-second cadence and 2400-second hard
timeout remain unchanged. Independently captured final scoped tests cover the
reviewed signed mount implementation.

All eight actual qualification contract inputs and raw manifest digest remain
unchanged. No server, expensive workload, product candidate or qualification
run was started. All-thread checks, durable operator authority, production
preparer/start/backend, controlled host enrollment and release evidence remain
closed. See the
[production audit and server checklist](../../production-readiness-and-server-launch-20261011.md).

Run the verifier from the repository using Python 3:
`python docs/testing/performance/0.74/local-runs/production-readiness-local-20261011/verify-packet.py`.
It only reads files; it does not contact a server, refresh policy or open admission.

Final doc/0.74 local contract, book/link and scoped format checks pass. Their three
closed logs were added after the 21-capture checkpoint printed by the earlier
verifier; that historical output is retained, not rewritten to say 24.
