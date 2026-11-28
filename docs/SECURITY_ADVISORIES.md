# Security advisories

Receive suspected vulnerabilities through [private reporting](../SECURITY.md).
Keep the report and proposed fix private until maintainers coordinate disclosure.

## For a confirmed vulnerability

1. Verify a fixed release and upload the corrected versions of affected crates.
2. Publish a GitHub Security Advisory in the Rust ecosystem, identify the
   crates.io packages and their affected and fixed versions, and request a CVE
   where appropriate. Include a link to the fix and the supported upgrade path.
3. Submit a Markdown advisory to [RustSec](https://github.com/RustSec/advisory-db)
   under `crates/<package>/`. Use one advisory per affected crate and reference
   the GHSA and CVE in `aliases` when assigned. Use
   `RUSTSEC-0000-0000` for the submission placeholder.
4. Yank affected crates.io versions in coordination with the fix. Yanking keeps
   existing Cargo lockfiles usable. An advisory remains necessary to notify
   those users.
5. Add the advisory and the fixed version to the release notes.

The repository owner enables private reporting and performs the registry and
advisory actions. The templates, validation and workflows can be tested locally.

## Fork advisories

Review upstream advisories from the [weekly check](../.github/workflows/fork-upstream-advisories.yml) against
the fork's actual source revision, enabled features and patched code. The
[fork decisions](security/fork-decisions.md) identify the branches under review.
When an issue applies, publish an advisory under the `hydra-*` package name.
Use `related` for the upstream advisory and use `aliases` for an upstream CVE
when it identifies the same vulnerability. Record non-applicability with the
source revision, rationale and evidence in the fork decision.

The weekly check scans the upstream identities recorded in
[`forks.toml`](security/forks.toml). Its findings are inputs for triage.
The release and consumer RustSec gates scan the real resolved lockfile.

When retiring a fork, announce its replacement and migration path, then submit
an informational RustSec advisory with `informational = "unmaintained"`.

## Templates and checks

- [Fork vulnerability](security/advisory-templates/hydra-fork-vulnerability.md)
- [Fork end of maintenance](security/advisory-templates/hydra-fork-unmaintained.md)
- [HydraCache vulnerability](security/advisory-templates/hydracache-vulnerability.md)

Use the prerelease test to verify the range with the exact boundary version
before submitting an advisory. For a different prerelease base version, check
that base separately using its own affected and fixed examples. Run
`python3 -B -m unittest scripts.security.test_fork_advisory` from the repository
root with the pinned `cargo-audit` version installed.
