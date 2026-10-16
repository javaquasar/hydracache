# Original collector pipes under acknowledged Starting

Implementation `a533e6df10ca67d2681fc0df0edcdc22b65610ac` adds real anonymous
kernel pipes, not a product process or root sink. The consuming entry retains the
same acknowledged Starting, original authority/receiver/storage and common fence.
No caller FD/path/time, raw-FD export, Stdio conversion, retry or start is exposed.
Two distinct pipes retain CLOEXEC, nonblocking readers and blocking writers.

Original full checks bracket kernel inspections. FIFO identity, dev/inode,
owner/mode/link count, exact status/access flags and CLOEXEC remain bound.
The acknowledgement hook precedes final authority/kernel closing checks.
Partial creation errors keep refused original ownership and all transferred
descriptors until drop. Pipes are declared before Starting so their fields drop
before the original fence; the healthy joined control checks actual read EOF
after owner drop, not just an empty queue.

## Tests

Both normal GNU release test binaries passed one serial nonroot matrix:

| Cases per compiler | Count |
| --- | ---: |
| Construction interruptions with retained partial ownership | 6 |
| Descriptor, status, CLOEXEC, inode and mode substitutions | 8 |
| Original trust and concurrent shared-OFD flag drift | 2 |
| Trust change at acknowledgement | 1 |
| Already-refused Starting, no pipe allocation | 1 |
| Healthy owner and both private EOFs after drop | 1 |
| Total | 19 |

All descriptor cases require pipe Invalid, not coincident original Time refusal.
The concurrent case changes the shared writer flags on another thread after the
first inspection; successful original validation must precede the closing refusal.
Every chain starts fresh. Failure keeps Busy and retained history, whose fixed
admission refuses after drop. No failed server scenario was rerun or waived.

Per compiler: four actual-kernel unit tests, one positive named-owner API test,
4096 randomized flag values plus six edges with seed `0x07407011`, 57 selected
compile-fail examples and 63 collector/IPC/lease/fence/consent/host regressions pass.
The local suite passes 187 static contracts on Rust1.94; current/MSRV Windows and
Linux check/strict Clippy compile the full affected targets. Scoped formatting,
the WSL all-workspace formatting check, registry180, doc-check and the local
nonpromotable performance contract pass. Ship admission remains expected-red.

## Custody and retained observations

`matrix-*.log` are full raw SCP copies. Build-time Git-blob observations match
the committed ten source/gate files. Binary hashes are pinned in the runner.
These unsigned prebuilt test observations are not a signed production receipt.

The missing-entry E0599 and the earlier baseline collector control are retained.
Intermediate release builds were not dispatched: review moved the acknowledgement
hook before the closing checks, strengthened failure classification and corrected
field drop ordering before the final pinned build. Earlier fast checks did not
prove that ordering; the final static and joined drop controls do.

The first independent postflight assembly accidentally omitted its shell function.
Its observed exit was zero despite `preflight: command not found`, so its printed
success is explicitly not accepted as verification. Its raw observation is kept.
The corrected strict-errexit postflight includes and runs the complete function.
The matrix itself had already run five successful pre/post guards; it was not
rerun to repair this reporting error.

Temporary root `/tmp/hc074-original-pipes-20261016.eO13aX` and existing helpers'
separate owned `/tmp/hydracache-authority-*` leaves contain only synthetic fixtures.
All modeled host paths remain below their private roots. No production directory,
global campaign lock, policy/epoch, service, installation or clock was mutated.
Boot, supervisor PID8839/start/restarts0, installed binary/receipt and absent active
campaign/lifecycle/authority contexts are checked before/between/after the lanes.

## Scope and next producer

Run `powershell -NoProfile -File ./verify.ps1` here. It verifies artifact hashes,
case/command/source inventories, compiler/seed counts and historical Git blobs.
The earlier Starting/SIGKILL and pre-consent packets still verify independently.

Private unexported pipe closure is not all-writer revocation. This owner provides
no root collector, sink, pump, original executable binding, manager/process/cgroup
handoff, watchdog, completion, later cell, new crash proof or enrolled server execution.
Do not replace these originals with pipes returned by the fixture collector.
Next consume this same owner into the private sole-root sink/start path, keeping
the original windows and requiring independent production trust/enrollment.
Actual start and cessation must be joined before completion or cells2–4.
