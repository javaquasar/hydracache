# Offline native Linux preparation and workspace formatting

A separate full checkout is prepared on local WSL ext4, with an independent
object database and every tracked file checked. The first actually dispatched
workspace format-check in that context passes under the original 120-second
diagnostic bound. The incomplete GNU frontier and full package remain separate
gates; neither is dispatched by this preparation.

Implementation: `636feb166b273b5c3d6fc0b93c4fccf57560cf03`.

## Complete offline transfer

The explicit clean source commit contains 53,842 tracked regular files,
2,140,731,448 blob bytes and 404 resolved ref/object-ID bindings. The exporter
reads the entire tree and refs, creates one complete offline bundle and checks
HEAD/status/refs again before publishing its request. No source or evidence
path is excluded. Two earlier captures are preflight-only, including the one
misnamed `native-export-first`; its command omitted `--export`, so no bundle
existed or attempt was overwritten. The distinct `native-export-once` is the
sole actual export.

Bundle bytes: 229,408,242; SHA256
`502abdb4bf01b164f40db1d650cfc6cabc6d766a15cae74506ea47a40fd4c01c`.
Request SHA256:
`44aaf01ea277b78a0ac3b2a77e382e61ac92ffebe0c4ba12a615eee5efd0e3e4`.
The bundle/request remain locally retained under
`target/native-frontier-20261027/636feb166b273b5c3d6fc0b93c4fccf57560cf03`.
The repository evidence retains the complete request inventory and receipts,
not another 229 MB bundle copy. This local transfer is not ship evidence.

Preparation imports those objects into an empty-template repository and checks
out the explicit detached source at:

```text
/home/artur/.cache/hydracache-074-frontier-native/636feb166b273b5c3d6fc0b93c4fccf57560cf03/repo
```

Every file's actual bytes must match its Git blob identity, with the expected
Git executable mode. The full native tree/ref mapping, clean status, qualifier
and connectivity pass. There are no alternates, shared worktree metadata or
network remote. Resolved ref/object IDs are reproduced; symbolic-ref topology,
source configuration and reflogs are not claimed to be mirrored. LFS pointers
remain exact tracked blobs, not downloaded large artifacts. These are sequential
transfer checks, not atomic filesystem attestation or production authority.

The actual materializer and existing heartbeat/companions are SHA-pinned before
launch. The unchanged Linux subreaper contains the one setup tree under a
separate 600-second preparation bound and five-second retirement budget. Its
receipt reports success, including owned-child retirement. No service or campaign
is stopped, installed or started. Windows source/HEAD postflight remains exact.

Capacity preflight reserves the full tree, two bundle copies, 512 MiB slack and
the unchanged 4 GiB floor. Both Linux and the physical Windows backing volume
are checked between major steps and every 256 file observations. The native
volume's large virtual free-space report never substitutes for physical space.
This reservation does not admit a later build's additional artifact growth.

## Controls and local scope review

Nine portable controls pass on Windows and in native Linux. One separate Linux
integration creates only an owned tiny temporary Git repository and verifies
independent checkout, refusal on reuse, byte drift and symlink substitution.
That fixture is not a project test. Early eight-case development captures remain
retained, but the then-untracked new scripts were not in the coordinator's source
map; only the final staged/committed source captures have that binding.

The three preparation modules and two control targets use only local filesystem,
hash/JSON parsing and fixed Git operations. The exporter refuses dirty/stale HEAD
or drift during export. Native preparation suppresses inherited Git execution
configuration for its own commands, disables hooks/maintenance and copies no
source Git configuration. Its only persistent writes are the exclusively created
native directory and owned transfer/receipt paths. There is no arbitrary command,
test dispatcher, resume, retry, recursive deletion or production-control API.
Failed partial directories and receipts are retained. No old live predicate,
current-source guard, conditional census read or original deadline is changed.

All-targets check, strict all-targets/all-features Clippy, Python compilation,
doc-check and the local structural performance contract are separate checks.
No Rust runtime or package test source changes in this step.

## First format preflight refusal and exact source correction

The first native format preflight refuses before starting Cargo: it incorrectly
compares native LF config bytes with the Windows CRLF working representation.
Read-only inspection proves the only difference is CRLF and both parse to the
same TOML alias. The first output and script witness are retained; no formatter
result is invented for this refused preflight.

The corrected preflight pins the complete native config from the exact source
commit, SHA256
`db18cd83f159c3152bcab6c10bbaac35be479c67a24b48c0f508bd1c7eca7a34`,
and independently retains the full Windows representation pin
`90f023744f969c176d98d811145b883b411da0fe8a294e930a0f965456ac49a3`.
It checks fresh full-tree bytes/status before and after formatting, rejects
inherited build/test overrides and shadow Cargo configuration, and uses only
an offline formatter child. No current body is learned as an approved pin.

The separate first actual native command is:

```text
cargo +1.94.0 fmt --all -- --check
```

It passes with no source change under the original 120-second bound and the
unchanged owned CI cleanup. This is a formatting verdict, not CPU attribution
or a native-storage speedup measurement. The old Windows `os error 206`, old
WSL format timeout and first config preflight refusal retain their own outcomes.

## Next validation and unchanged release boundary

The [evidence packet](local-runs/native-frontier-preparation-20261027/index.json)
retains commands, source maps, complete manifests, first outcomes, checked
launchers, native receipts and the successful full format result. Archived
scripts remain inert data. No GNU five-target cohort, full package or product
workload is started here.

Next review the actual native Git/build environment and GNU target effect scope,
including explicit LF-versus-Windows text representation and unchanged binary
auxiliary inputs. Only then admit one distinct native GNU frontier attempt under
the original 900-second/four-thread contract, not a replay of the old attempt.
After that, finish the current full-package coordinator review and dispatch under
its unchanged 780-second/four-thread/4-GiB/no-filter/no-retry contract.

Qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
All live checks/frequencies and 300/60/10 windows stay exact. Security policy,
original worker release/all-writer retirement, durable completion/cells 2–4,
reconnect, production trust and compatible-host admission remain open. No new
performance hypothesis, clock/service change, server run or qualification occurs.

## Subsequent degenerate path refusal

Final leaf review finds that `PurePosixPath('.')` has an empty component tuple,
so the initial pure selector accepts that degenerate name. A new regression
first records eight passes and one failure. The separate fix
`12f5f73c2f43a96a106afcf1af168a2efcf6d5ca` requires a nonempty component tuple;
all nine portable controls then pass on Windows and Linux. The
[separate first-result packet](local-runs/native-path-refusal-20261027/index.json)
binds both source versions to their pre-execution digests. No valid Git tree
entry is affected, and the already checked context remains exact at source
`636feb166b2`. Neither native materialization nor formatting is repeated. This
strengthens a new transfer validator, not any product/live predicate or window.
