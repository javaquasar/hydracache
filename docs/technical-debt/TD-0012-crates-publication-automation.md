# TD-0012: crates.io publication automation

Status: open proposal. This debt is documented here, but is intentionally not
registered as a release plan until the publication workflow is approved.

## Objective

Publish the release libraries to crates.io only when the exact tagged commit has passed the agreed release test tier. A tag must never publish a crate merely because the tag exists.

The current repository does not yet provide this guarantee:

- `.github/workflows/publish-release-notes.yml` creates or updates a GitHub Release, but does not run `cargo publish`;
- `.github/workflows/ci.yml` runs on `main`, `master`, pull requests, manual dispatch, and schedule, but not on tag pushes;
- several heavy and gated jobs run only for schedule/manual events;
- `.github/workflows/post-publish.yml` is manual and verifies a published version after the fact.

The publication flow therefore needs its own explicit release gate.

## Required release sequence

```text
version bump on main
  -> full release qualification for the candidate commit
  -> merge to protected main
  -> create immutable vX.Y.Z tag on that commit
  -> release qualification for the tag SHA
  -> package validation and crates.io publication
  -> consumer verification from crates.io
  -> GitHub Release notes
```

The tag-triggered workflow must publish only after all required jobs are green. The workflow must not treat skipped nightly/manual jobs as proof that they passed; the required release tier must be declared explicitly.

## Workflow changes

### 1. Release qualification workflow

Add a dedicated `.github/workflows/release-qualification.yml`, or extract the reusable test jobs from `ci.yml` into a reusable workflow and call it from the tag workflow.

The workflow should trigger on:

- `push.tags: v*.*.*`;
- restricted `workflow_dispatch` for re-running a known tag, without a bypass mode.

The validation job must:

1. resolve `vX.Y.Z` and verify strict SemVer syntax;
2. verify that the tag exists and points to the checked-out commit;
3. verify that the tagged commit is reachable from protected `main`;
4. compare the tag version with `[workspace.package].version` and every publishable package version;
5. verify the matching entry in `docs/plans/releases.toml` and the release notes file;
6. require the release status and evidence ledger to indicate that the release gate is complete;
7. reject a moved tag or a tag created from an unapproved branch.

The required test jobs should include, according to the release manifest:

- `cargo fmt --check`;
- workspace check/build/test with `--locked`;
- clippy and documentation checks;
- coverage ratchet;
- MSRV;
- performance budget;
- Redis compatibility/oracle tier;
- DST, Raft, snapshot and fault-injection gates;
- governance checks: `doc-check`, `verify-no-test-features`, `canary-check`, release evidence and gated-test registry checks.

Nightly-only or environment-dependent tests must either be promoted into this release tier or be recorded explicitly as non-blocking. They must not be silently counted as green.

### 2. Publish job

The publish job must have an explicit `needs` dependency on every blocking qualification job. It should run in the protected GitHub Environment `release` and require an approval for production publication.

Before publishing it must run:

```text
cargo metadata --locked
cargo package --locked for every publishable package
cargo publish --dry-run for every publishable package
```

Only after all dry-runs succeed may it execute the real `cargo publish` commands.

The job must be idempotent. Crates.io versions are immutable, so a retry must:

- detect an already published exact version;
- verify that the existing version matches the requested release;
- skip only that exact package/version;
- fail on a different or partially conflicting version;
- never attempt to overwrite an existing crate version.

### 3. Publication credentials

Configure one of the following outside the repository:

- a crates.io API token stored as the GitHub Actions secret `CARGO_REGISTRY_TOKEN`;
- a supported crates.io/GitHub trusted-publishing setup for the release workflow.

The token must be attached to the protected `release` environment, never committed to the repository, and never printed by a shell step. Workflow permissions should remain minimal (`contents: read`, plus only the permissions required to create the GitHub Release and the selected publishing mechanism).

## Package inventory and order

The workspace currently exposes 22 packages with default publication enabled:

```text
hydracache
hydracache-actuator-axum
hydracache-cdc-postgres
hydracache-client
hydracache-client-protocol
hydracache-client-transport-axum
hydracache-cluster
hydracache-cluster-chitchat
hydracache-cluster-raft
hydracache-cluster-transport-axum
hydracache-core
hydracache-db
hydracache-diesel
hydracache-macros
hydracache-observability
hydracache-redis-compat
hydracache-seaorm
hydracache-server
hydracache-sql-lint
hydracache-sqlx
hydracache-transport-nats
hydracache-transport-redis
```

The following packages are internal or test-only and must remain excluded because they set `publish = false`:

```text
hydracache-cache-sim
hydracache-cluster-testkit
hydracache-fuzz
hydracache-operator
hydracache-sandbox
hydracache-sim
hydracache-sim-wasm
xtask
```

The final workflow must derive the inventory with `cargo metadata` and validate it against a reviewed allowlist or generated release manifest. It must not blindly publish the entire workspace.

Publication order must be dependency-aware. The preferred implementation is a topological order generated from workspace package dependencies, followed by a bounded retry while crates.io index entries become available. A manually maintained order may be retained as a review artifact, but it must be checked against `cargo metadata` on every release.

## Package quality gates

For every publishable crate, the release gate must verify:

- package name and version;
- `license`, `repository`, `description`, `readme`, keywords and categories;
- included files from `cargo package --list`;
- no unintended local path dependency remains in the packaged artifact;
- package builds from the packaged tarball, not only from the workspace;
- package documentation builds with warnings denied where applicable.

The workspace version, `Cargo.lock`, release manifest and tag must describe one release. The current branch reports workspace version `0.63.0`; before publishing 0.64 the version bump must be present in the tagged commit.

## Post-publish verification

Convert `.github/workflows/post-publish.yml` from a manual-only check into the final job of the release flow, while retaining a restricted manual rerun option.

The verification must create a clean consumer project and resolve dependencies exclusively from crates.io at the exact release version. It must run:

- `cargo check`;
- `cargo test`;
- `cargo doc --no-deps` with warnings denied;
- dependency-order smoke checks;
- representative cache, cluster, Raft, transport, adapter and observability examples.

The current smoke list is useful but incomplete relative to the 22 publishable packages. The generated publication manifest should drive both publication and consumer verification so that a newly publishable crate cannot silently escape the post-publish test.

Crates.io publication cannot be rolled back automatically. If post-publish verification fails, the workflow must mark the release failed, open/annotate a release incident, and block further version promotion; it must not attempt to republish or overwrite the same version.

## GitHub repository controls

Configure the repository so that:

- `main` is protected and release tags can only be created from an approved commit;
- `v*.*.*` tags are protected from deletion and force-move;
- the `release` environment has required reviewers;
- the publish workflow cannot be approved through an untrusted pull request;
- the GitHub Release is created only after crates.io publication and post-publish verification;
- release workflow logs redact registry credentials;
- the release workflow publishes a machine-readable evidence artifact with commit SHA, tag, package list, package checksums, test jobs and publication results.

## Proposed implementation units

1. Add release version/tag/evidence validation and a generated publish manifest.
2. Extract or add the tag-specific release qualification workflow.
3. Add dry-run, dependency-aware, idempotent crates.io publication.
4. Add protected credentials and GitHub Environment configuration documentation.
5. Connect post-publish consumer verification to the publication result.
6. Make GitHub Release notes depend on successful crate publication and verification.
7. Add a release runbook and a failure/retry procedure.

## Definition of done

- A tag on a non-main or version-inconsistent commit cannot publish.
- A failing blocking test prevents the publish job from starting.
- All publishable packages pass package and dry-run validation.
- Internal/test-only packages are excluded mechanically.
- A retry is safe for already published exact versions and fails loudly on conflicts.
- A clean consumer project verifies the published artifacts from crates.io.
- GitHub Release notes are created only after successful crate publication and verification.
- The complete release evidence is downloadable from the workflow run.
