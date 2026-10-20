# Native consuming holder recheck after allocation changes

Both authorized native matrices refuse the first healthy case during
`prepare_owned_root_guardians`, before placement or consuming join. Neither
lane completes a case. The native composition gate remains false; no matrix is
retried and no following case is substituted from another cohort.

| Compiler | Original exit | Causal expiry age | First custody observation age |
| --- | --- | --- | --- |
| GNU 1.94 | 101 | 10,003,954,927 ns | 9,176,614,214 ns |
| MSRV 1.88 | 101 | 10,068,587,556 ns | 9,237,420,814 ns |

The original refusal is `diagnostic_collector_causal.rs:109`; both kernel
refusal records are null. The unchanged test unwrap at
`consuming_held_live.rs:243:65` locates this refusal inside owned-root preparation.
These are admission observations, not throughput or comparative latency
estimates. Different failure frontiers in the earlier native cohort do not
establish a regression, speedup or clock actor. The first custody records already
use over nine seconds of the original ten-second causal window; attribution
must cover preparation before and through that frontier, not unreached held-role
hashing. The accepted allocation controls do not establish execution readiness.

## Exact inputs and one protected attempt

The [plan](../../held-native-recheck-plan-20261020.md) freezes starting source
`1ecaa895fdf84114f441f95f04c6411e9389096a`. Selected runtime, tests, source
guards and qualification are identical to reviewed image source
`b5f1b8fce6b3046239087873cb85ff1bfc557f0f`. Normal GNU/MSRV images are reused
from the [PID control](../pid-path-cost-20261020/README.md) and preceding
[local composition recheck](../held-composition-recheck-20261020/README.md), not
rebuilt. Their exact SHA256 values and nine original bundle hashes are in
`inputs.sha256`. The bundle executable is read for material validation, not
launched; held roles and issuer identities remain synthetic test data.

Preregistration is `fccac032`; protected staging is `72cf0e3c`. Upload
`/tmp/hc074-native-recheck-upload-20261020.OiTtzO` is admin-owned 0700. Execution
`/tmp/hc074-native-recheck-root-20261020.hBFgSO` is root-owned 0700, with
runner/images 0500 and bundle members 0400. Root checks every copied input,
independently pinned runner/manifest hashes and ownership before launching only
the protected runner, once, with a clean environment. Wrapper PID is 266408.
The historical runner differs only in its permitted temporary-root prefix; all
preflight checks, original deadlines, selectors and one-shot latch are retained.

Each lane uses the exact full selector, one test thread, no selected-case/cost
override, no profiling and no retry. Wrapper exit is 1, its postflight exit 0.
The later four cases on each lane are not attempted. SSH staging and all four
recorded control phases exit zero; test failures remain separate from transport.

The private namespace projects only the fixture's unique cgroup subtree over
/sys/fs/cgroup. It never mutates host system.slice or sends original manager
start. All mutation outside those existing fixtures is limited to new staging
and export directories. There is no release installation, clock/time-service
change, supervisor operation, product workload or qualification run.

## Host preservation and raw transfer

All nine campaign/lifecycle markers are absent before staging, around both
matrices and in independent postflight. Boot remains
`e81671d6-bf23-4ebf-b8fe-5dad1f45866f`; supervisor stays active/running with PID
8839 and NRestarts zero. Installed executable SHA256 stays
`f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62` and receipt
SHA256 stays `b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525`.
Host mount tables before/after are byte-identical and no owned fixture root or
cgroup remains. These are point-in-time checks, not a continuous host reservation
or proof that every clock observation was stable.

The root-generated export
`/tmp/hc074-native-recheck-export-20261020.nI8uQm` contains 17 completed raw
log/exit files plus a hash manifest. SCP byte hashes match that manifest and
the separate root export transcript. SSH stdout/stderr captures are decoded
text, not signed custody receipts or byte-exact external streams. The fixture's
`server=false` describes its historical vocabulary; this outer receipt correctly
records `server=true`. No binaries, credentials or private keys are committed.
Staging, export and local binary-transfer copies are retained, not broadly removed.

## Verification and next gate

`verify.ps1` derives exact original refusal/age/frontier records from raw logs,
checks root-copy hashes and all host preflights, verifies raw exports, freezes
selected source and qualification, and rehashes three prior sealed packets.
Strict inventory rejects missing, extra, duplicate, changed or reparse members;
summary mutations refuse. `-Review` cannot bypass an existing inventory.
`checks.ps1` records local format/docs/registry/non-promotable contract checks.
Targeted runtime tests and strict lint are inherited only from identical source
and hash-bound images; they are not counted as new test executions.

The next local work is observation-only attribution of preparation up to the
first custody check and through owned-root closing validation, retaining every
live check and original origin/deadline. This isolated server authorization is
consumed; another native invocation needs separate permission. Healthy native
composition, joined SIGKILL/lost-ACK, authenticated delivery, all-writer cessation,
durable completion/later cells, reconnect and real production enrollment remain
independent closed gates. Earlier native/WSL failures are not reclassified.
