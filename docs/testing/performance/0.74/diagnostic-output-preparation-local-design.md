# Fenced fixture output preparation

## Scope

The baseline is `ba3b2fc48d0acb05dd81a3dc82576e056967d54b`.
The next local W11 slice prepares empty named streams for an already reserved
diagnostic cell. It is a fixture API only, not a production output provisioner,
signed IPC operation or unit launcher. The production inspector and worker
account authentication remain separate prerequisites.

## Hypothesis and order

Holding the existing `.host-execution.lock` while comparing the exact active
state and creating a cell through retained directory descriptors should prevent
cooperating campaign, diagnostic and duplicate preparers from interleaving.
Capture the complete supervisor baseline before adding tests, capture the
missing-API red result, implement, then retain focused and regression results.
These are safety checks, not performance measurements.

## Fixture boundary

Require an absolute path without dot, empty or symlink components. Reject both
production prefixes `/var/lib/hydracache-performance` and
`/opt/hydracache-performance`. The fixture root must be mode 0700 and owned by
the actual non-root effective UID and GID. No caller-supplied worker identity,
chown, account creation or NSS authentication is permitted.

Open the existing lock without following links and retain its descriptor through
the entire operation. Require a regular single-link lock owned by the fixture
identity without group/world write or special mode bits. Use the same exclusive
fs2 lock as the existing coordinator. Recheck original root and lock bindings
around state observation and mutation; create descendants with `mkdirat` and
streams with exclusive `openat`, relative to retained directories.

Require exact persisted state equality, stage Reserved and an unexpired asserted
boot/monotonic clock, including the existing ten-second controller-loss limit.
Reject campaign markers, pending diagnostic/IPC documents and either fixed
lifecycle fixture context. No revision, duration, heartbeat or controller state
is changed. Boot and clock arguments remain fixture assertions.

Release the lock explicitly before returning success, propagating an unlock
error. Error paths also attempt an explicit unlock during Drop. Closing one
descriptor alone is insufficient when another descriptor retains the same open
file description. A deterministic cloned-descriptor test covers this without
forking a test process; it is not a production inherited-FD audit.

## Exclusive preparation and failure

Only fixed paths `diagnostic-fixture-outputs/<lease>/<surface>/stdout.json` and
`stderr.log` are created. Output and lease directories may be reused only with
the exact fixture ownership and 0700 mode; a cell must be new. Streams must be
new regular single-link 0600 empty files. Sync the streams and directory entries.
Return the existing read-only fixture named-output guard, never file descriptors
or a production guard.

Existing cells, including complete earlier preparations, refuse; there is no
automatic replay, unlink, truncate, cleanup or repair. Inject failures after
creating the cell and each stream and retain their partial filesystem state.
An empty output/lease parent may be reused after failure before cell creation;
it contains no cell intent or started work. Different valid cells remain serial
under the shared host lock.

## Required tests and remaining boundaries

Cover all four surfaces, exact state/revision mismatch, terminal or expired
state, clock regression and boot mismatch, campaign/context/pending conflicts,
shared lock contention, concurrent duplicate preparation, symlink/hardlink/mode
refusal and partial-state retention without overwrite. Revalidate the returned
guard and show refusal on subsequent path substitution.

This cooperative temporary fixture does not prove adversarial same-UID namespace
continuity, crash recovery after power loss, a whole-operation deadline, signed
authorization, authenticated worker enrollment or production ownership transfer.
There is no persistent preparation receipt/schema or admission flag. Production
preparation requires a separate root-owned capability and enrolled worker policy;
no install, process, systemd, server, observer or qualification route is enabled.

## Local result

The [retained packet](local-runs/diagnostic-output-preparation-20261010/manifest.json)
keeps the complete 158-pass Linux baseline before test addition. The initial
missing-API attempt also triggered the rustc diagnostic renderer; the formatted
repeat refused on 27 missing-API errors without that renderer panic. Ten initial
behavior tests passed. The first broader parallel run refused with Busy in
fixture coordinator setup; its precise scheduling cause was not traced.

A separate cloned-descriptor regression failed deterministically before explicit
unlock. The corrected fixture uses the retained fence for reservation too, with
no retry of Busy and no change to the existing coordinator or qualification
tooling. An accidental field placement compile failure and an enum-name strict
lint refusal are retained separately rather than labelled expected safety reds.

Final coverage passes 172 Linux tests, one pre-existing ignored and three further
fourteen-test repetitions. Portable Windows coverage passes 28, with zero Linux
preparation tests. Root contracts/evidence/governance pass 121 checks before
packet registration; packet registration receives its own skeleton check.
Affected checks, strict lint and scoped format use 1.94.0. No full workspace
milestone verification or performance workload is claimed.
