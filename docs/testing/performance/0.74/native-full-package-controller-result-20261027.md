# Full native package controller and resource refusal

The bounded GNU coordinator is implemented and its focused controls pass. The
full package remains incomplete: both native Cargo dispatches stop during
compilation, before any test target starts. The second stops on the unchanged
physical disk floor. Neither result replaces the earlier Windows V9 timeout or
the separate five-target GNU success.

Accepted implementation: `25c85d5ecc3936039a8f2650f1f459567aacdcf2`.
Prototype implementation: `d9c9594482551783683e8c869d9317eb0021b89d`.
Native package source: `636feb166b273b5c3d6fc0b93c4fccf57560cf03`.

## Fixed command and ownership

The coordinator preserves the complete default package selection:

```text
/home/artur/.cargo/bin/cargo +1.94.0 test --offline --locked --no-fail-fast
  --target-dir /home/artur/.cache/hydracache-074-194-check
  -p xtask --lib --tests -- --test-threads=4
```

The bound remains 780 seconds including compilation. The child receives only
the three explicit environment overrides `RUSTUP_AUTO_INSTALL=0`,
`CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=2`; the parent restores its environment.
Cargo stdout and stderr inherit separate exclusive files, independently of
controller heartbeats. The unchanged, hash-pinned Linux CI group retains original
pidfds, sole-subreaper custody, nonreaping leader observation and kernel ECHILD
before retirement. Its cleanup budget remains five seconds. This containment
does not establish production cessation of all writer aliases.

Fresh full source/effect/environment review precedes the timed command. The
accepted controller additionally requires the exact green, unfiltered Linux
control command and source hashes before Cargo can start. Missing, failed,
filtered or stale controls refuse dispatch. There is no deadline, selection or
retry CLI override. Both fixed output directories now exist and cannot be reused.

## First defects and outcomes

The strengthened descriptor-restoration test initially compared entire `fstat`
tuples. A log reader changed access time, although descriptor identity was
unchanged. That first Linux failure is retained. A shell chain incorrectly
continued after the failed test and dispatched the prototype. Compilation had
already begun when the exact controller received SIGINT through its retained
pidfd; no test target started. The controller reports `cancelled`, exit 125,
`retired=true`, no cleanup errors and a complete native source/ref postflight.
This was a dispatch mistake, not a passing gate or a refusal before Cargo.

The corrected oracle compares device, inode, mode and available device identity,
not mutable timestamps or file size. Its first Windows version then exposed an
unavailable `st_rdev` attribute. That error is also retained; no package command
followed it. The portable correction and mandatory source-bound control gate
pass six Windows cases with three Linux-only skips and all nine Linux cases.
Real synthetic Linux fixtures verify separate streams, natural owned exit,
timeout retirement of a detached grandchild and resource refusal after spawn.

The accepted controller then passes fresh native review and starts compilation.
Its second outcome is:

| Observation | Retained result |
| --- | --- |
| Inner classification and exit | `resource-floor`, 125 |
| Bound and cleanup budget | 780 seconds and 5 seconds |
| Original Linux CI group | `retired=true`, kernel ECHILD established |
| Cargo stdout | Empty; no libtest target or case starts |
| Cargo stderr | Compilation message only |
| Outer capture | `disk-floor`, exit 1; Windows source and HEAD unchanged |
| Inner final result and accounting | Absent; not synthesized |
| Full native postflight for this attempt | Incomplete |

The outer floor guard terminates its WSL/coordinator client during postflight,
after the inner controller has recorded retirement. This is a resource refusal,
not a test failure or package pass. All 153 test targets remain unstarted in this
attempt. The floor is sampled, not a hard allocation quota: physical free space
falls below 4 GiB before cleanup completes. Do not claim the floor was held
continuously or that missing postflight succeeded.

## Capacity and retained evidence

The earlier approximately 6.2 GiB free-space snapshot did not provide sufficient
compilation reserve. The subsequent physical snapshot is approximately 2.66 GiB,
below the original 4 GiB floor. Further builds/tests remain blocked on capacity;
neither another sample nor a larger deadline fixes that prerequisite.

The unused Windows `target/debug/incremental` cache contains 2,835,268,439 bytes
in 2,232 files. Its exact root, parent chain and entries were checked for aliases,
and no Windows compiler was active. The tool rejected its scoped deletion before
execution: nothing was removed and no alternate deletion mechanism was used.
After capacity is repaired, check both the backing volume and native filesystem
with additional compilation reserve before admitting a distinct attempt. Clearing
that cache alone is not a build-capacity guarantee.

The [127-member packet](local-runs/native-full-controller-20261027/index.json)
contains 60 content-addressed objects. Index SHA256:
`6db5f78599a0bd404b9dce1d06310cb7e607cfe698cbe1ca88a570ca7c482b02`.
Both attempts, every control outcome and all exact early controller/test source
witnesses remain available. An append-only scope correction explicitly records
two actual Cargo dispatches and preserves the initial scope and collection index;
no raw outcome is rewritten. Archived scripts are data, never executed.

The preceding offline step passed ten pure controls on each platform, doc-check,
Python syntax checks and the local non-promotable performance contract. The new
controller's final focused controls passed before the resource refusal. Subsequent
documentation/evidence work uses only metadata, hash and staged-byte checks;
there is no fresh Rust gate or package run below the floor.

## Remaining release boundary

Restore capacity, preserve these consumed attempts and separately admit a new
attempt with current green controls and the same command, bounds and accounting.
No unchanged Windows replay, suffix joining or automatic retry is allowed.
Required Windows/full-package coverage, remaining security policy, worker release
beyond ReadyObserved, all-writer cessation, durable completion/cells 2–4,
reconnection and production trust/compatible-host admission remain open under
the [closure plan](release-closure-execution-plan-20261026.md).

No new performance hypothesis, server operation, service or clock change,
release installation, product load or qualification occurs. Cargo.lock and the
qualification manifest remain unchanged, as do all live checks and the original
300/60/10 origins and windows. No product speedup or release readiness follows
from this controller work.
