# Native preparation attribution before another holder gate

The first native holder attempt expired at age 10,015,008,749 ns while preparing
owned-root guardians. Its first custody check was already at 9,027,489,393 ns.
Those age samples cannot distinguish CPU, filesystem work and descheduling.
The failed attempt and the four unexecuted cases remain unchanged.

## One separate diagnostic session

Run the already pinned GNU image `ca5484aa951414b80aefd0148c7d8f5185a8fdf73bb197cf98a790de59b8cff1`
once with `HC074_CONSUMING_HELD_LIVE_CASE=healthy` and
`HC074_CONSUMING_HELD_COST=1`. Existing test-only observers report phase wall
time, process CPU, authentication, overlapping kernel/live spans, custody
boundaries and validation rounds. They add no authorization predicate or guard
clock, and do not replace the original checks. Instrumentation overhead remains
part of the observation, not a product latency or successful admission claim.

`profile_session.py` derives the session from the exact frozen holder safety
frame. It preserves boot, stable supervisor PID/restarts, nine absent campaign
markers, installed binary/receipt digests, input ownership/hashes, private
fixture leftovers and byte-identical mount postflight. New paths and a new
exclusive `profile-once` marker separate it from the consumed holder attempt.
The image's 191 source inputs and nine material inputs must match before upload.

Only synthetic root fixtures in a new protected `/tmp` tree and their unique
private cgroup/mount composition are in scope. No systemd activation, original
manager send, installation, service/clock change or product workload occurs.
Retain the first result even if it fails before all phases. No retry or automatic
continuation follows. Original 300/60/10-second windows and their origins remain;
the unchanged 1200-second outer timeout is only a process safety bound.

## Analysis and next decision

Compare phase spans only within this diagnostic execution. Inclusive nested
spans overlap and must not be summed as disjoint time; process CPU excludes
children. A wall-minus-process-CPU residual is not an I/O attribution. Report
any first refusal with its original origin and age; a clean postflight cannot
promote the failed phase or the incomplete matrix.

Select one confirmed cost owner, record its baseline, and implement one isolated
candidate with differential/fault controls. Fresh kernel observations, their
count/order, all live predicates and original windows must remain intact. A
candidate needs a distinct gate; neither this profile nor old negative evidence
can serve as its successful original delivery or all-writer retirement proof.
