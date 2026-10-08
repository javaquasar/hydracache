# Rented Linux host: read-only inventory and prepared diagnostic pilot

The user authorized **read-only host preflight and preparation**, not execution
of a numerical pilot, service mutation or qualification. The inventory ran over
ordinary SSH with the reviewed Python source on stdin; nothing was uploaded,
installed, built or measured as a product workload. Protected campaign paths
required `sudo -n` read access. No secret configuration, private key, environment
or process command line was read. The collector tests passed nine checks on
Windows and nine on local Ubuntu/WSL2, including a real dangling-symlink fixture
on Linux. An initial static safety test falsely matched `/environ` in prose;
the comment was clarified before the collector commit. No workload was involved.

The retained inventory uses clean collector commit
`62114be0f5da3218706e30d7424acfb5d0579d07`; the source was checked clean before
and after collection. Earlier exploratory SSH checks and the development
inventory were read-only too, but are not substituted for this source-bound
receipt. See `local-runs/rental-preflight-62114be0/manifest.json` for exact byte
hashes. This is an SSH diagnostic receipt, not independently signed host
admission or qualification evidence.

## Observed state

- Online CPUs `0-7`, isolated and nohz-full `1-4`, SMT off; one NUMA node.
  The eight scaling governors reported `performance`. This is kernel inventory,
  not proof of future child placement, IRQ isolation or a reserved host.
- Total RAM `66,490,490,880` bytes; available `65,503,191,040` bytes at capture.
  Free disk was about 421 GB. Availability is transient, not a capacity claim.
- The active-campaign marker, campaign-lifecycle context and controller-loss
  context were absent at both lifecycle snapshots. Process enumeration was
  complete at those instants; the only matching process was supervisor PID 8839.
  The runner was inactive; the supervisor stayed active with zero restarts.
  Two snapshots do not exclude a process starting between or after them.
- Installed supervisor/fixture hashes matched the provisioning receipt:
  `f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62`.
  Installed source remains `543108f1ccd206ae670803c2617f07e7fa91ae62`, distinct
  from the new observer source. No provisioning/update was performed.
- Rust/cargo 1.94.0 are installed for the existing build account, although root's
  audit PATH has neither. Future builds must select `+1.94.0` explicitly.
- A single fixed two-second CPU-counter sample was quiet. Its raw ticks are
  retained, including iowait/steal; this is **not** A/A noise calibration.
- glibc 2.39 and C/make/CMake/perf/taskset are present.
  `perf_event_paranoid=4`; unprivileged stack/hardware-counter profiling access
  is unresolved. No sysctl or privilege policy was changed.
- `ldconfig` found no registered jemalloc/mimalloc/tcmalloc libraries. This does
  not exclude static builds or establish any provider's telemetry capability.
  System allocator active/resident/retained remain **unproven**, not a green gate.

## Prepared P0, not a launched A/A or A/B cohort

`rental-diagnostic-pilot-contract.toml` and four strict JSON configs prepare a
small, baseline-only CPU-window feasibility probe. Fixed order: embedded,
direct ClientSurfaceState, RESP2, RESP3; GET, 16 keys, 4 KiB values, seed 740074,
eight slots, 10k original offers at 5k/s, 64 excluded warmup calls. RESP uses
pipeline 10; native has no pipeline framing. Every config has the same corpus
digest and explicit queue/deadline/SLO/histogram bounds. A no-fixture Rust test
checks the actual observer input/corpus implementation, not a copied Python
benchmark algorithm.

Planned execution is at most four fresh baseline processes, serially on CPU 1,
60 seconds per child tree and 300 seconds total, no concurrent builds and no
counting allocator. Neither those proposed limits nor a successful config
validation authorize execution. Linux binary/source/lock/features, coordinator
deadline/receipt handling, exclusive host reservation and CPU/noise preconditions
must first be reviewed and sealed. Existing supervisor and runner are untouched;
no shell pilot may bypass their active-campaign protection.

P0 stops on the first invalid result, including CPU or measurement wall below
the unchanged one-second floors, errors, missing samples, undrained owners,
overflow, placement drift or background activity. It retains the failed attempt;
it does not retry with a higher work count or lower floor. A short native window
would be evidence for a **new preregistered bounded/streaming driver**, not a
native regression, noisy-host conclusion or failed product candidate.

The following are separate subsequent contracts, not parts silently enabled by
this draft: finite five-pair A/A and A/B cohorts, fixed private PKI across secure
fresh processes, the complete native/RESP matrix and supported System-retention
telemetry. The closed W9e allocator replacement/purge experiment is not reopened.
Jemalloc diagnostic counters cannot certify retention of the shipping System
allocator. No performance winner, full D3 pass, C74 freeze or numerical release
claim is inferred. The qualification manifest and historical sealed packets are
unchanged.

## Local preparation checks

Windows/Rust 1.94.0: all 65 performance-contract checks and all 13
release-evidence integration checks passed. The new no-fixture pilot input
test passed in default and get-owner builds; the existing CLI validated all
four configs with `fixture_started=false`. Observer and xtask all-target checks,
strict all-feature clippy, formatting, doc-check, the local non-promotable
performance contract and 17 governance checks passed. Ship admission remains
expected-red for unresolved C74 and qualification. These preparation checks do
not certify a full observer/workspace suite or a numerical host cohort.

Two initial Cargo invocations used a nonexistent release-evidence target and
then an irrelevant zero-test lib filter. Neither is counted as a passing suite;
the corrected `--test release_evidence` invocation executed all 13 checks.

## Subsequent local preparation, still no host execution

The [preparation implementation and exact-source packet](rental-pilot-coordinator-design.md)
add strict four-cell identity validation and metadata-only Linux build inspection.
The successful inspection fixture is fake, not a real Linux release binary.
Eleven tests passed per local OS. The external host-lock reservation idea was
rejected before a live attempt: Busy propagates out of installed-source
maintenance and can invoke the service's restart policy. Supervisor changes or
a controlled pause need explicit scope. No additional host SSH, build, fixture,
service operation or numerical attempt was made in this preparation step.
