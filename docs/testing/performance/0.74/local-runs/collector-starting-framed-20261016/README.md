# Durable Starting publication with early framed consent

Implementation `a3f2734e687c98e9d7b37f16250a1cc96ad0cae9`. Approved preparation/acceptance implementation
`77b3ad15` and its independent [48-case semantic packet](../preconsent-frame-20261016/README.md)
precede this join. The [earlier rejected publisher](../collector-starting-publication-20261016/README.md)
remains immutable negative evidence.

The publisher, private storage, crash harness, nine stage cuts, AfterReturn,
three repetitions, seed `0x074057d1` and public API test are exact restorations
of archived candidate A. Only the fixture's prepare/accept calls select the
new constant-depth path. The static contract reconstructs that exact archive
and all narrow adapters against `77b3ad15`. Nineteen full two-round live checks
remain around the write boundaries and acknowledgement; no signature cache,
renewed origin, new deadline, skipped guard or failed-case retry.

## Observed gate

Both normal GNU release test binaries passed once, serially as UID/GID1000:

| Gate | Rust 1.94 | Rust 1.88 |
| --- | ---: | ---: |
| Injected storage interruptions | 9 | 9 |
| Live trust/storage/inventory drifts | 9 | 9 |
| Healthy fault control | 1 | 1 |
| Actual SIGKILL terminal outcomes | 30 | 30 |
| AfterReturn lost-runtime-ack kills, included above | 3 | 3 |
| Uninterrupted crash controls | 3 | 3 |
| Independent fresh-process inspections | 66 | 66 |

The parent's pidfd pins its own child; terminal status must be SIGKILL.
Each new child constructs its own actual receiver and complete original chain
after exec. Busy exclusion is checked while it lives and release after death.
Each retained history is inspected twice by new processes. Original history,
context and authority snapshots include bytes, inode, ownership, mode, link
count and modification/change times; inspections preserve them and refuse
fixed-context admission, never reconstructing the disappeared owner.

AfterReturn is reached only after the publisher returned an acknowledged runtime
owner and that owner revalidated. Killing there proves loss of that runtime
acknowledgement/owner, not loss of an external protocol ACK. Even complete sealed,
fsynced bytes do not recover it. File/directory sync is exercised successfully;
power loss, reboot, storage EIO, production enrollment, global replay closure
and legacy bypass closure are not proved.

## Local checks and unchanged host

Each compiler passes ten fast units, two public API tests, fifty selected
compile-fail examples and 62 IPC/lease/fence/host/consent regressions. Current/MSRV
Windows and Linux checks and strict Clippy pass, as do both 186-contract suites,
WSL formatting, registry179, doc-check and local nonpromotable performance
contract. After the join, six preparation compile-fail examples, its named
API and 3072 unchanged clock edges also pass per compiler. The expected-red ship
guard still rejects incomplete release identity,
qualification and receipts; this is not a release-wide green result.

Preserved negative observations: the missing publisher API fails E0599 before
implementation; the first static restoration refuses a formatter-expanded
assertion, corrected by its exact approved adapter; Windows workspace fmt refuses
OS error206 (command-line length), and unchanged WSL fmt passes. No failed server
scenario was discarded or rerun.

Runner root `/tmp/hc074-starting-framed-20261016.dAn2pE` holds only unsigned
prebuilt test binaries, runner and raw logs. Existing fixture grammar also
allocates owned `/tmp/hydracache-collector-starting-crash-*` and
`/tmp/hydracache-authority-*` leaves, with modeled host paths under their origin.
This is not signed build custody, an installed receipt or an enrolled worker.

Read-only checks before/between/after all four lanes preserve boot,
supervisor PID8839/start/restarts0 and installed binary/receipt hashes; all listed
production campaign/lifecycle/authority contexts remain absent. No service stop,
installation, clock change, product load, Actions dispatch, pilot or expensive
qualification occurred.

## Recheck and remaining gates

Run `powershell -NoProfile -File ./verify.ps1` here. It hashes raw artifacts,
checks every compiler/repeat/stage/inspection, command outcomes, historical Git
blobs and unchanged qualification/kernel/deadline inputs. Evidence remains valid
when later work changes the working copy.

Four negative offline manifest canaries reject a server-ready claim, a changed
lost-ack count, an artifact hash and a source blob. Their observed failures and
the exact pre-selfcheck verifier snapshot are retained separately. Replay those
historical 43-artifact canary inputs with `verifier-before-selfcheck.ps1`; the
final verifier additionally inventories these selfcheck artifacts.

This closes first-cell durable intent/process-loss publication only. Still join
independently selected production trust/epoch and original controller admission,
private output/executable/streams/process/cgroup start, autonomous watchdog and
all-writer cessation before completion or cells2–4. Then perform the separately
admitted signed installation and nonproduct integrated lifecycle rehearsal.
No server or numerical qualification admission is opened by this packet.
