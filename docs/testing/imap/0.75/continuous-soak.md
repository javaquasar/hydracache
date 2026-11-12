# Continuous local development segments

Preparation builds the fixed reference worker, current local daemon and pinned
native JARs. A segment keeps its service/process between rounds. The controller
records PID/platform birth/incarnation independently of the worker's reported PID
and incarnation. Replacement or controller restart ends that segment. A parent
link always starts a new segment; durations are never added together.

```powershell
cargo xtask imap-pre074-tools --release 0.75
cargo xtask imap-soak --release 0.75 --cohort all --profile smoke --host-budget-seconds 180 --output target/imap-075/continuous-smoke
cargo xtask imap-soak --release 0.75 --cohort reference --profile six-hours --host-budget-seconds 22000 --output target/imap-075/continuous-reference-6h
cargo xtask imap-soak --release 0.75 --cohort native --profile day --host-budget-seconds 90000 --output target/imap-075/continuous-native-24h
cargo xtask imap-soak-cancel --release 0.75 --checkpoint target/imap-075/continuous-native-24h/native/checkpoint.json
```

Use a clean source tree and fresh output directory. `reference`, `native` and
`daemon` have separate capability/resource units. Native counts describe fixture
ownership. Reference temporary credits must be zero at quiescence; the fixed
28-byte retirement fence is persistent state. Daemon rounds verify HC1/RESP
interoperation, invalidate the controlled key and observe absence. Unavailable
internal daemon owners remain null.

Active duration excludes readiness/warmup. Hard/startup/drain/output/round bounds
are independent; host eligibility includes cleanup reserve. Short proofs remain
capped at 60 seconds. The separate soak policy permits at most 26h. Long profiles
require one explicit cohort and eligible host. Implementing 6h/24h profiles does
not assert that those runs occurred.

`checkpoint.json` binds config/controller; `identity.json` captures observed worker
birth at readiness. Bounded `stdout.jsonl` records logical progress. `report.json`
records the terminal reason, transcript hash and actual process cleanup. Heap,
RSS, GC, CPU and thread samples are diagnostics; unavailable samples are null.

Cancellation writes an owned bounded marker, allows drain and then enforces
bounded process cleanup. Missing drain, leaks, artifact/runtime drift or failed
cleanup cannot complete. `--resume <report-or-checkpoint>` validates the old
context and creates a new segment. Live owners cannot transfer to a new controller.
Corrupt/partial progress refuses recovery; preserve it for diagnosis.

The first manual CI workflow admits one hour on the isolated `hydracache-imap075`
self-hosted runner label and this 0.75 branch. No PR, fork or scheduled trigger is
enabled. The frozen 0.71 topology remains unchanged; a separate 0.75 extension
registers this workflow. The main qualification matrix keeps its 45-minute budget.

Manual reports remain in the owned runner directory. Automatic transfer of those
reports/build logs to GitHub Artifact is not configured: approval review rejected
that transfer. The runner owner must preserve/review the local archive and apply
the intended retention policy; this workflow does not claim automatic remote
retention or delete earlier runs. Preparation smoke reports continue to be bound
by the existing preparation receipt contract.

Longer profiles use the CLI on an eligible host. They are not advertised as one
6h/24h Actions step: GitHub documents a
[360-minute step limit and 24-hour GITHUB_TOKEN lifetime](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax).
Future long-job orchestration must preserve the same controller/workers across
monitoring steps and arrange final archive credentials/retention. Restarting a
worker for each monitoring step is not continuous qualification.
