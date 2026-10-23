# Eighteen role child readiness and local fault checks

The separate --systemd-bootstrap-held-readiness-v1 entry now passes local owned
process controls on GNU 1.94 and MSRV 1.88. It validates eighteen inherited roles,
answers one authenticated readiness challenge and remains nonexecuting until an
original-bound stop condition. It does not release or execute a worker. Readiness
does not establish original manager delivery, production enrollment, all-writer
cessation, durable completion or server admission.

## Implementation

The [plan](bootstrap-held-entry-plan-20261023.md) preserves the historical child
modes. Exact additive source views restore their complete bodies, the old
installed-reader body and its controls to commit 55c69519. Historical full role,
kernel, manager and admission validation files retain their preceding source
contracts. The frozen qualification manifest remains unchanged.

The focused files are:

- diagnostic_held_activation.rs: exactly five canonical activation variables,
  fifteen fixed names and the separate readiness argument. Missing, foreign,
  duplicate, non-UTF8 or malformed values refuse. These are consistency inputs,
  not enrollment credentials.
- diagnostic_bootstrap_held_child.rs and diagnostic_bootstrap_held_inventory.rs:
  a closed FD0..17 census and the old FD0..16 predicates in their original order.
  The new FD17 must be a distinct nonblocking Unix seqpacket endpoint with
  PASSCRED and PASSPIDFD. Only FD17 changes from inherited flags to CLOEXEC;
  the old roles are not repaired or remapped.
- diagnostic_bootstrap_held_mode.rs and diagnostic_bootstrap_held_transport.rs:
  fixed-slot adoption, one-shot readiness and a nonexecuting wait on the original
  creator/guardian pidfds, absolute alarm and readiness channel. No alarm is
  created or rearmed by this entry. Disconnect, duplicate data, errors and expiry
  stop readiness; no worker release, exec or fallback follows.
- diagnostic_held_child_exchange.rs: a separate framed method learns only the
  nonce from the authenticated packet. Expectations otherwise come from the same
  observed sealed entry and actual child PID. Full role frames run before
  receive, after receive and after the only possible ACK; refusal is irreversible.
- diagnostic_held_delivery_wire.rs: the parent additionally sets
  SetLoginEnvironment=false and a finite UnsetEnvironment list. Arbitrary child
  environment is not silently cleared. Unsupported manager properties must
  refuse without fallback.

The old release channel remains empty, and HCBT0741, HCBI0741, HCHR0741 and
HCHA0741 bytes are unchanged. The versioned [compatibility contract](../../../COMPAT.md#held-child-readiness-entry-v1)
describes the additive entry and explicitly supersedes only the earlier
candidate's unactivated-argument statement. All original 300/60/10 origins,
windows and live checks remain intact.

## Fault controls

The integration fixture copies the check binary into an owned temporary
filesystem inode, retains its sealed reference and entry, and inherits the
fixed role set. This is synthetic consistency evidence, not a signed installed
origin. It creates no service, namespace or cgroup. RAII kills and reaps only
owned helpers, including assertion-failure paths.

The controls cover a valid ACK with a live kernel-delivered peer pidfd; duplicate
challenge; missing/extra/swapped roles; record offset and wrong executable inode;
foreign/missing environment, wrong PID/count/names/invocation/argument; mutated
entry/context/manager/client/invocation/deadline/child PID/reserved bytes; wrong
kernel sender and malformed nonce; guardian loss, channel shutdown and original
alarm expiry. Stdout stays empty and the original release queue stays empty.

The SIGKILL control kills and reaps its own child before readiness and after an
ACK is queued but not consumed. Before the latter kill, a bounded MSG_PEEK checks
the exact ACK bytes without removing them. An independently held pidfd for that
owned helper is live before kill and readable after reap, and the exit signal
must be SIGKILL. Late authenticated reception is refused on this local kernel;
queued payload is never promoted into recovered readiness or completion. No
child is restarted and no challenge is resent. The alarm control separately
checks that the original timerfd is readable after expiry without reading or
rearming it. These controls are not the complete original-start SIGKILL matrix.

## First refusals and corrections

The historical absence baseline failed with exit 101 and is retained without
rerunning it after implementation. The first Windows source-contract lane
refused because the appended compatibility block left one extra blank line
after exact removal. Removing that extra line restored the entire old document;
the historical comparison was not weakened.

The first GNU integration lane passed eleven controls and failed four readiness
scenarios before ACK. The old assertion-only fixture intentionally uses equal
manager/client hashes; the existing readiness codec requires distinct endpoints.
Only the new fixture now supplies distinct hashes and reseals its matching
records, without changing any origin or deadline. The old fixtures are untouched,
and a dedicated new control retains refusal of their equal-name assertions.

The next GNU/MSRV matrix passed. A stronger lost-ACK oracle then exposed an
incorrect test assumption: after SIGKILL and reap, the strict ancillary parser
returned Invalid instead of delivering a late authenticated ACK. Both compiler
lanes retain that first failure. The corrected oracle checks queued bytes with
MSG_PEEK while the child is alive and requires the late refusal after reap.
The original parser, live-sender requirement and runtime checks are unchanged.
GNU and MSRV here share one local WSL kernel, not two independent kernel setups.

## Passing gates and retained evidence

The four final lost-ack lanes pass the same selected 173-file snapshot, pinned
to checked commit 530e197ed2e56a7db1c01916b9ed40beed25217e:

- Windows: 70 source contracts and 26 bootstrap/codec/build controls; all-target
  check, all-feature strict Clippy and formatting.
- GNU 1.94 and MSRV 1.88: 153 selected test invocations per compiler, including
  all seventeen integration controls, all-target check and all-feature strict
  Clippy. The entry filter overlaps the complete child-unit run; these are not
  153 distinct test functions.
- Metadata: six Python runner/retention controls, doc-check, 188 registered
  gates, the nonpromotable local performance contract and diff checks.

The [evidence packet](local-runs/bootstrap-held-entry-20261023/audit.json) retains
fourteen lanes, including all five refused lanes and their raw logs, exits and
source snapshots. The 2,733 members plus inventory were independently verified
as exactly 2,734 staged Git blobs before evidence commit 05e03381. Inventory
SHA256 is 1fa54ddf1788e7c925c32b5a9f8bad8a12bdccda105e491e72eb6ccaae8d3adc.
The qualification SHA256 is still
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

The selected unsigned GNU check image is 1,680,168 bytes with SHA256
4750a12568c4293ad8db6ce356abb272e36ea2eb81d6c1639c55f1a74947dfce.
It was built from the checked working sources before their commits, not from a
clean signed production build receipt. No installation, original live attempt,
clock/service change, server action or product workload belongs to this gate.
Elapsed test times are not preparation-cost or admission-window-fit measurements.

Commits: plan 63ec86a1; entry and necessary controls 7621d02f; runner/retention
530e197e; immutable evidence 05e03381. The report and article are separate docs.

## Next integration boundary

The next step is positive delivery through the original manager and the consumed
owner, including actual activation environment, loaded-unit hardening and exact
process/executable/streams/cgroup observations around readiness. It requires a
separately permitted local service fixture; this no-service gate did not run it.
Authenticated original worker start, cessation of every writer before fence
release, durable completion/later cells and the full original-path SIGKILL/lost-ACK
matrix remain separate gates. Server installation and rehearsal are not enabled
by these local readiness results.
