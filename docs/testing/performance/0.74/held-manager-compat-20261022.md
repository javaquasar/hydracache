# Local manager rejects the extra descriptor transport

The real local WSL systemd 255.4-1ubuntu8.18 manager rejects ExtraFileDescriptors.
The single StartTransientUnit returns the exact unsupported-property error,
and its unique temporary unit is absent before and after. This establishes a
local transport blocker, not original delivery or cessation of all writers.
The joined original fault matrix remains closed and was not launched.

## Implemented compatibility control

`tools/held-manager-compat-074/probe.py` implements a bounded, fail-closed test
entry, not a production launch API. It accepts only a fresh attempt name in the
exact local root WSL workspace. It binds clean committed source, input hashes,
busctl hash, boot, namespaces and unchanged qualification. Bus queries resolve
a unique systemd name and require its reported PID 1/UID 0 before addressing it.
The same identity is checked again at the end. Separate busctl connections are
not misrepresented as the retained original manager channel or full live frame.

The fixed mutation uses one random hc074-fdcompat-<128-bit-hex>.service name,
mode fail, six fixed properties, an empty extra-FD array and /usr/bin/false as
the only possible helper. The existing-unit guard rejects collisions. There is
no arbitrary command, path, property, destination, stop/reset, fallback or
restart route. The helper cannot be a product worker and receives no writer,
config or executable FD. Each possible command is recorded without overwriting.

The per-call 250 ms bus timeout, at-most-two-second process guard and ten-second
whole-test guard bound this compatibility control only. It creates no original
reservation and does not reset an admission origin. All original live checks,
300/60/10 windows, first Causal refusals and qualification remain unchanged.
Existing supervisor/runtime/worker code and policy were not modified.

## First outcomes and the single mutation

| Source | Retained outcome | StartTransientUnit calls |
| --- | --- | ---: |
| aabcb360 | First read-only response exposes the tool's scalar-versus-argument-array decoder error | 0 |
| 88dbabf6 | Correct decoding reaches Version but exposes a wrong property interface argument | 0 |
| 13dcce83 | Exact manager and context checks pass; property rejected; exact unit absent before/after | 1 |

The first two source versions and raw failures remain immutable. The decoder
correction replays the captured actual reply and rejects malformed alternatives.
An exact-argv regression fails before correcting the fixed property interface;
that red baseline is retained too. Neither tooling correction resends a failed
mutation, replaces an original attempt or changes deadlines. After the sole
StartTransientUnit the lane stops; no second submission or fallback is attempted.

The final manager's response is:

```text
Call failed: Cannot set property ExtraFileDescriptors, or unknown property.
```

The addressed owner is :1.2 with bus-reported PID 1/UID 0. Final source, boot,
namespaces, UID/GID, qualification and tool hashes agree at opening/closing;
boot is 5b0642d5-1412-4620-9daa-2bf089c93fff. The three tooling invocations have
distinct recorded WSL lifetimes and are not concatenated into a continuous run.
No WSL restart, clock or service adjustment is issued by the test.

The exact name hc074-fdcompat-f16cffa4263cb63e12f95d1c730b09e6.service is not
loaded before or after. No temporary slice, held launcher or active writer is
created. No existing service is stopped, restarted, reconfigured or upgraded;
no server operation, product workload or performance cohort occurs.

## Why permissions do not resolve this blocker

The [v255 service setter](https://github.com/systemd/systemd/blob/v255/src/core/dbus-service.c)
does not implement ExtraFileDescriptors; [v257](https://github.com/systemd/systemd/blob/v257/src/core/dbus-service.c)
does. The native extra-FD bridge under consideration therefore cannot be assumed
on this host. This is not proof that every possible held transport needs 257,
nor that a newer version alone would satisfy held-image, stream, process or
descriptor placement. The currently materialized eleven launcher slots are
not a runnable manager specification by themselves.

Two decisions remain possible: provide a separate local environment with the
required native FD API, leaving this Ubuntu/systemd untouched; or review a new
bootstrap/FD-handoff protocol compatible with 255. The latter introduces another
receiver and copies outside the attempt cgroup, so it must bind original peer,
image, owner, deadline and guardians and account for those copies through EOF.
It cannot be added silently as a compatibility fallback. Neither decision is
implemented or authorized by this negative control.

After that selection, original durable one-shot uncertainty, retained-channel
delivery, actual held process/barrier binding, active collector and all-writer
terminal proof still need implementation and their positive/fault matrices.
Full-frame checks and original absolute deadlines must remain intact. Completion,
cells 2–4, lost-ACK/reconnect and production enrollment/server admission stay
separate; a rejected property cannot count as any of those successes.

## Tests and evidence

Eleven final offline controls pass on both Windows and Linux. They cover exact
query/launch argv, captured reply shape, unsafe names, wrong owner/version,
preexisting/uncertain unit, manager churn, malformed/error/lost replies, timeout
receipt retention and refusal to resume. They launch no actual service. Raw
native evidence contains 15 command receipts across the three invocations and
exactly one StartTransientUnit. Offline replay reconstructs the final observation
from the recorded replies without contacting a bus.

The [packet](local-runs/held-manager-compat-20261022/) seals 62 members plus
inventory.json, SHA-256
`de30658692b85efa826e5af7af320abdda3768fe7f22453f1bb9f2efd9f6c4b8`.
All 63 files match staged raw Git bytes. It retains first failures, all source
versions, three preregistered plans, the red regression baseline and check logs.
Implementation commits are aabcb360, 88dbabf6 and 13dcce83; evidence is a770c361.
This is not a full release qualification, installed-source receipt or production
trust proof. The original qualification SHA-256 is still
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

Formatting, doc-check, all 186 discovered gates registered, local non-promotable
performance-contract and diff checks pass. Their receipts and checkout document
hashes are retained in the separate [documentation packet](local-runs/held-manager-compat-doc-followup-20261022/).
No Rust source changes require a new compiler/lint cohort here; no fresh Rust
build, performance measurement or full release qualification is claimed.
