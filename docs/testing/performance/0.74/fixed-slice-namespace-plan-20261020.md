# Fixed slice kernel checks in an isolated mount namespace

Baseline is 9724e428b9850df8ff5aae34caa47ad8ba18e702. The existing fixed slice
models and read-only host anchor check do not execute a positive FixedSlice pin.
The next local test exercises the unchanged production observer on real cgroup2
objects without creating a systemd unit or changing the host system.slice.

The root harness creates one uniquely named empty temporary cgroup subtree
directly below the local WSL cgroup mount. Its synthetic system.slice and derived
attempt child are below that temporary root. A separately execed test child first
unshares its mount namespace and makes propagation private, then bind mounts only
that temporary subtree at its private /sys/fs/cgroup. The harness remains outside
the subtree and in its original mount namespace. No cgroup.procs, cgroup.kill,
systemd channel, original attempt, timer or product executable is used.

The child must accept an actual empty PinnedRoot and FixedSlice, retain read-only
CLOEXEC observer descriptors and reject a different attempt. Independent fresh
cases replace the named attempt root, replace system.slice, change root mode,
change parent mode, change the mount anchor, mark the original root refused and
substitute an identically named root under a different parent. Each case starts
with a successful pin/revalidation and checks the intended refusal. Restore
temporary names/modes before the parent removes only its exact empty directories.
Child failure or a bounded harness timeout must still leave parent cleanup active.

This is a kernel observer fixture, not a successful original-owner join. The
private mount is deliberately not a systemd-owned slice. Manager ownership,
exclusive population, authenticated launcher/watchdog/collector objects, held
projection, delivery, all-writer cessation and subsequent lifecycle gates remain.
No production install path, schema, predicate, full frame, effect bracket or
original 300/60/10-second origin/deadline changes. No server, supervisor, system
bus, product workload or qualification is authorized.

Retain baseline/image identity, a missing-module compile refusal, current GNU
and MSRV exact test images and raw case outputs. Add a manual non-shipping gate
and exact additive adapters which reconstruct all 437 old supervisor sources and
the old registry, refusing missing/duplicate hooks and unrelated drift. Run the
existing nine fixed-slice/placement ordinary controls, new fixture, strict lint,
scoped formatting, documentation, registry and local performance contract checks.
Do not use a zero-case invocation as success or reseal earlier evidence packets.
