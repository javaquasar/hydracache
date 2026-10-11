# Local signed intent journal evidence

This packet retains local fixture safety checks for durable signed consumption,
not a production backend, execution capability or release qualification receipt.
Baseline is `c690a08b3a52a6545e0d1ea27ef6188e8e6aed16`, preregistration is
`53de447fb01ba1e034d562f55d5e6d01ee3e687a`, implementation is
`875e89bfbc1e5248db9d7ea71950fc5b10c1decc` (tree
`2447a9633b7d1146e4309c8fa53c6e672f5e6799`). Raw closed captures and source hashes
are bound in [manifest.json](manifest.json); [verify-packet.py](verify-packet.py)
checks them read-only from the repository with Python 3.11 or newer.

## Checked result

Nineteen new unit tests pass three serial repetitions. The final selected Linux
scope passes 471 tests with one existing ignored; native Windows portable tests
pass 195. Six compile-fail doctests pass. Root checks cover 107 performance
contract, 13 release evidence, six gated-target and 23 governance cases. Strict
Linux/Windows lint, MSRV 1.88, Rust 1.94 checks/rustdoc, formatting, docs, book and
local non-promotable performance contracts pass. Totals are harness rows for the
named scopes, not unique release coverage. No new performance workload is run.

The protocol checks exact persisted Reserved state, original guard and history
under a cooperative host fence. It retains full signed bytes in four exclusive
record/marker pairs with file/directory fsync; stale boot/controller binding,
duplicate cell/nonce/request, unsafe metadata, partial publication and original
object replacement refuse. Late complete unacknowledged publication stays consumed.
Inspection is data, not execution authority. Details: [design](../../diagnostic-intent-journal-local-design.md).

## Retained unsuccessful attempts

`02` is absent-API red. `04` is a Rust compiler ICE during incomplete test
development, not an executed test; `06` is a JoinHandle test compile error.
`08` records early fixture failures because its parent did not have the required
0700 mode. `07`, despite its old filename, is an ineffective green regression:
the expected refusal occurred before its target checkpoint and proves no inode
replacement protection. After fixing the fixture and asserting checkpoint reach,
`09` is the genuine red regression. Created-descriptor metadata checks make the
final replacement cases green. Intermediate 18-test repeats and 470-test scope
remain distinct from final 19-test repeats and 471-test scope.

`37` is a command-selection error naming nonexistent root test targets; `38` and
`45` retain corrected root coverage. `41` is expected historical working-source
drift: the earlier projection verifier still pins its earlier implementation.
The current verifier checks that earlier packet's raw hashes and source at its
recorded commit without modifying it or claiming unchanged current code.

Qualification dry-run (`40`) and ship admission (`42`) remain expected-red. The
frozen manifest and all eight inputs are unchanged from baseline. Its main plan
has committed LF digest `cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828`,
not the declared `6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d`.
This inherited mismatch needs reviewed reconciliation, not a silent digest update.

## Remaining boundary

Positive fixtures use nonroot temporary paths, synthetic keys and asserted
peer/time/boot/completed-cell values. Deduplication is limited to one retained
journal. Physical power-loss, owner/root tampering, snapshot rollback, live peer
and clock authentication, real reboot, actual once-only execution and I/O
deadlines are not established. A fixed root-owned backend/original-authority join,
material/output/runtime composition, Starting/watchdog/owned cleanup and real
issuer/host enrollment remain before launch. No frozen 0.73 artifact, host,
workflow, workload, qualification input, threshold or admission state is changed.
