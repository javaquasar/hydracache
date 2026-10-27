# Offline native Linux frontier preparation

This tool prepares a separate full checkout on local WSL ext4. It does not run
tests, workload, a service or a release installation. It cannot grant package,
original-execution or qualification admission. Prior timeouts remain retained.

The Windows exporter requires an explicit exact clean HEAD, inventories every
tracked blob and every ref, and creates a complete offline Git bundle. It checks
HEAD/index/status/refs again before publishing the SHA-bound request. No paths,
evidence or histories are pruned. Git LFS pointers remain Git-tracked bytes; this
is not an LFS artifact download. Global/system Git configuration is suppressed
only for preparation; hooks/fsmonitor/LFS processing and repository maintenance
are disabled per command, without changing the source repository configuration.

The nonroot Linux materializer requires the fixed ext4 parent, owned nonsymlink
ancestors and an exclusively created commit-named directory. It verifies request
and bundle bytes, creates an empty-template repository, imports exact refs,
checks out the explicit detached commit and checks **every** tracked file's raw
blob identity and Git executable mode. Connectivity, clean status, qualifier,
absence of alternates and independent `.git` are required. No network remote or
shared worktree/object database is installed. These are sequential local transfer
checks, not atomic host attestation or production enrollment.

Materialization runs under the unchanged SHA-checked CI heartbeat/subreaper,
with a separate 600-second setup bound and the original five-second retirement
budget. The heartbeat owns only its spawned preparation tree, not any service.
Source requests, native receipts and failed partial directories are retained;
there is no resume, retry or recursive deletion API.

Both Linux and the physical Windows backing volume retain the 4 GiB floor.
Upfront reservation includes the full tree, two bundle copies and 512 MiB slack;
checks repeat between effects and every 256 file observations. The Linux virtual
free-space report cannot replace the physical floor. This is preparation only,
not a build-space guarantee.

```text
python -m unittest discover -s tools/native-frontier-074 -p test_contract.py -v
# Local Linux only, under the reviewed heartbeat:
python3 -m unittest discover -s tools/native-frontier-074 -p test_native.py -v
python tools/native-frontier-074/prepare.py --source FULL_SHA
python tools/native-frontier-074/prepare.py --source FULL_SHA --export
```

The default exporter invocation is preflight-only. Before materialization,
independently review/pin the three preparation bodies and unchanged heartbeat
companions, then supply the exact request SHA to `native.py`. No build/test
dispatcher is provided. A native source/environment effect review and explicit
separate attempt still precede GNU frontier or workspace formatting. Keep the
original 900-second/four-thread GNU and 780-second/four-thread/full-package
contracts; do not join old suffixes or reinterpret old refusals.

The [first complete preparation and format result](../../docs/testing/performance/0.74/native-frontier-preparation-result-20261027.md)
retains all transfer metadata and first outcomes. The native source is a separate
detached copy, not a mirror of source Git configuration/reflogs/symbolic-ref
topology. The 404 bindings are resolved ref/object IDs; file modes mean Git
executable identities, not production UID/GID or account enrollment. The full
native format-check passes after a separately retained LF/CRLF preflight refusal.
GNU frontier and full package admission remain closed.
