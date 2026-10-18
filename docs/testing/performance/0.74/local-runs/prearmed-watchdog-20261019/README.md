# Prearmed watchdog ownership and process fault evidence

This package tests private local guardian mechanics before original delivery.
The guardian is active, not a sleeping liveness surrogate. It retains a real
exclusive fence, creating-owner pidfd and original absolute BOOTTIME alarm
before Pending; it retains the enrolled target pidfd before Armed. The
[plan](../../prearmed-watchdog-plan-20261019.md) starts at `a44618c2`; plan commits
`5c350aae` and `5d6f561d` precede their implementation. The
[contract](../../prearmed-watchdog-contract.toml) keeps historical original
admission, whole-cgroup revocation, production construction and qualification
closed. No server operation is part of this packet.

## Ownership and acknowledgement

The private versioned 16-byte seqpacket protocol requires exact shape/version,
kernel SCM_CREDENTIALS and one atomically CLOEXEC target pidfd. The receiving
guardian checks actual creating PID/real UID/GID and a live direct child using
bounded local procfs reads and pidfd signal-0. The parent API also checks its
creating thread/effective credentials; SCM_CREDENTIALS itself does not bind TID.
Received rights are always closed on rejection, without signaling an untrusted
target. Retaining an accepted target precedes Armed. Sender uncertainty precedes
send, so missing ACK, send error or refusal cannot allow another enrollment.

After release the owned target writes one constant byte to a nonblocking pipe
and parks without further owner or deadline monitoring. The external nonroot
subreaper kills the actual creating process and separately observes target
SIGKILL status, reader EOF and fence exclusion. It receives exact pidfds and
kernel credentials, checks direct parent and executable identity, then cleans
up only owned/adopted children. Temporary fixtures use `/tmp/hc074-prearmed-*`;
no systemd manager, workload executable or product evidence sink is involved.

## Twelve fault cases and expected negatives

Each compiler runs the same seed `0x0740a019` and twelve cases:

- Pending owner loss: no target exists; guardian retains the fence beyond the
  original fixture deadline.
- Armed owner loss, peer loss, original expiry and duplicate wire enrollment:
  the released target receives actual SIGKILL and its pipe reaches EOF.
- Hidden writer: target death alone does not produce EOF; closing the driver's
  exact additional write alias is separately required.
- Lost ACK: guardian stops after retaining the target but before Armed; actual
  owner SIGKILL and guardian resume terminate the unreleased target without an
  effect, retry or fence release.
- Foreign sender and wrong right: actual copied-socket credentials or a regular
  file refuse; the untrusted target remains alive and unreleased at refusal.
- Guardian death before release: the barrier refuses and no effect occurs,
  but the last-holder flock is released. This is an expected negative.
- Guardian death after release: the target and its write FD remain live while
  the last-holder flock becomes available. This is an expected negative, not
  proof of custody or safe completion.
- Guardian stopped after release: after actual owner SIGKILL the target remains
  live with its writer at the original deadline. Resuming the guardian then
  causes actual SIGKILL and EOF, while its fence remains retained throughout.
  A live guardian pidfd is not responsiveness or an independently enforced timer.

The guardian has no completion or unlock API. Revoking means a signal attempt,
not observed cessation. The hidden alias is deliberately not revoked by this
one-target prototype; no all-writer guarantee is claimed. Neither EOF nor ACK
can promote the prototype to original execution authority.

## Baseline and retained preliminary results

`baseline-gnu194.log` passes the prior nine ordinary two-phase controls with one
ignored fixture; `baseline-barrier.log` passes the prior twelve kernel cases.
`red-module.log` records the expected missing-module compile failure before
implementation. The early units and `armed-owner-first.log` are development
checks, not selected-source receipts.

`matrix-gnu194-first.log` and `matrix-gnu188-first.log` pass the preliminary ten
cases before adding the two post-release guardian-failure controls. Their test
images precede a Clippy-only equivalent spelling correction. The first GNU 1.94
strict lint rejects a manual remainder check in `clippy-gnu194.log`; the code
uses `is_multiple_of` instead, without a lint allowance or policy change.
Corrected preliminary strict lint passes on both GNU compilers. All raw earlier
logs remain retained, including that lint failure.
`previous-packet-inventory.log` prints successful old-packet verification, but
its outer shell incorrectly checked stale LASTEXITCODE after a PowerShell
script. `previous-packet-inventory-confirmed.log` repeats it through a native
pwsh process and checks its actual exit code. The prior packet remains unchanged.

## Selected source and checks

Selected implementation is `ddcd8aa89bc235603666e860a6b4f0f8a5996e0b`.
The `gnu194-source` and `gnu188-source` receipts bind that same commit, nonroot
UID, compiler and equal before/after hashes of each executed libtest image.
Each cohort runs all-target checking, strict all-feature Clippy, fourteen
ordinary two-phase controls, twelve new process fault cases, the prior twelve
barrier cases and thirty old exchange controls. Ignored private-bus fixtures
are not counted as executed.

Separate Windows receipts cover 49 portable units, 17 compile-fail API controls
and strict affected Clippy. The complete source contract suite covers 265 tests,
including exact restoration of all 366 previous supervisor Rust bodies and old
compatibility/registry/imported-harness bytes. Mutation controls reject changed
registrations and old predicates. Formatting, gate registry, doc-check and the
local non-promotable performance contract have separate receipts. This is not
a whole-workspace milestone or product performance qualification.

The qualification manifest remains unchanged at
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The closed inventory hashes canonical UTF-8/LF/no-BOM text, independent of Git's
Windows newline conversion; image receipts retain actual binary SHA-256.
`./verify.ps1 -SelfTest` checks the inventory, cohort source/image binding and
eight in-memory negative controls. It grants no runtime authority.

## Reproduction

From the repository's Windows/nonroot WSL workstation with clean selected source:

```powershell
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188
```

Run the script from this evidence folder or use its full path. It refuses dirty
selected sources, existing receipt directories, failed/empty gates, source or
image drift and changed qualification bytes. A reproduction creates a new cohort
outside the original closed inventory; do not overwrite old receipts or count
an extended directory as the original packet.
`reproduction-guards.log` separately checks both scripts' syntax and refusal of
an existing cohort, path-escaping label and unsupported compiler without
starting a child fixture.

## Changed files and next integration

Private implementation, unit controls and subprocess fixture are
`diagnostic_prearmed_watchdog.rs`, `diagnostic_prearmed_watchdog_tests.rs` and
`diagnostic_prearmed_watchdog_fixture.rs` under the supervisor source directory;
the existing held-barrier module only registers the new private child.
`tests/prearmed_watchdog.py` is the external owned-process driver. The new
`performance_contract_074_prearmed.rs` guard is registered through the previous
two-phase guard. Compatibility, manual gate registration, local contract, plan,
collector design and optimization article record its limits.

The ambient procfs/direct-child check cannot admit a systemd-managed target
normally parented to PID 1. Join the retained original kernel/manager/cgroup,
signed artifacts and original streams inside the complete original validation
frame instead of removing that check or exposing a public constructor. Preserve
the existing original collector/custodian owners and add independent revocation
that survives guardian death or stop. Cover every writer/FD copy, observe both
actual EOFs and finalization, then address durable completion, later cells and
reconnect. All original 46 effect brackets and 300/60/10-second origins remain.
No release installation, supervisor operation, original/system-bus send, product
workload, dedicated-server operation or expensive qualification occurred here.
