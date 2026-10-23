# Original manager service: first local outcomes

Original delivery is **not yet demonstrated**. The new opt-in fixture implements
an owner-consuming chain through projection, materialization, signed bootstrap
selection, installed origin, held delivery and child readiness. Its first local
test refused at unit-absence preflight, before creating an attempt unit or
collecting an admission/consent observation. There was no manager delivery,
worker release, execution, completion, fence-release or server admission.

## Implementation and boundaries

The [plan](bootstrap-original-service-plan-20261023.md) separates this fixture
from the old private-cgroup-mount fixture. Only the new fixture may create one
previously absent, marked transient slice on the real system.slice tree. The
service request still comes from prepare_original_held_delivery and its retained
FixedManagerChannel, not from a reconstructed InstalledEntryUnit or busctl.
Preparatory slice control and cleanup are separate, explicitly non-authoritative
fixture operations. Every production validation body and 300/60/10 origin/window
is unchanged; exact historical source contracts check this.

held_original_service_documents.rs prepares a test-only signed bootstrap
policy/receipt against the same held policy, controller, epoch, host and launcher
digests. Its enrollment belongs to the original fixture's private protected
trust root, not production /etc. The real diagnostic bootstrap is temporarily
placed at the fixed /opt path after an absence check. Its reference is readonly
and sealed. Synthetic build assertions are not a clean production attestation.

The user explicitly approved a password-locked, no-home temporary account with
UID/GID 986 and supplementary GID 987. The runner guards names and numeric IDs,
copies account database originals privately, records the exact installed inode
and digest, and removes only owned artifacts after writer cessation. A successful
cleanup is not itself evidence that the original all-writer retirement gate ran.

## Preserved first outcomes

1. Initial account preparation failed: Debian rejects CREATE_MAIL_SPOOL as a
   login.defs override passed through useradd -K. No account, image or libtest
   invocation existed. Two empty owned groups remained because the conservative
   cleanup treated uncertain account creation as a failure. Their exact names,
   empty membership and GIDs were checked; both were subsequently removed.
2. That failure also exposed a WSL persistence boundary. /tmp is tmpfs: recovery
   copies disappeared across distro init while the shared VM boot_id stayed the
   same and the mount namespace changed. Original pre-failure backup-file bytes
   could not be reverified after that loss; this packet does not claim otherwise.
   The corrected runner uses protected /var/tmp on ext4. No clock, mask, existing
   service or WSL configuration was changed.
3. The account-compatible candidate used useradd --system without the invalid
   override. Its **first and only original-owner test invocation** exited 101 at
   the absence guard: systemctl show reported a synthesized slice as loaded.
   Causal observations and all three age-site call counts were zero; no scope
   creation intent or send marker was emitted. The full first stderr is retained.
4. A separate read-only control confirmed that GetUnit returns not loaded for
   an unused slice while systemctl show reports loaded. The isolated correction
   now uses GetUnit, accepts only the exact C-locale missing-unit response, and
   refuses disconnection, permission errors, wrong units, extra output or unknown
   status. Cleanup uses the same non-loading distinction. This corrected lookup
   has offline controls, not a repeated original-owner live pass.

The compatible candidate ended with test_rc=101 and cleanup_rc=0. Original
account entries were byte-compared after removal, account-tool backups restored
to that candidate's baseline, the bootstrap and created parent directories
removed. UID/GIDs, mailbox and fixed diagnostic image are absent. Pre/postflight
boot, manager namespace and supervisor snapshot bytes match within that session;
the supervisor was absent in this isolated distro, not stopped or restarted.

## Verification and evidence

Windows passed 77 selected source contracts and 26 component controls. GNU 1.94
and MSRV 1.88 each passed 154 selected test invocations for the composed fixture
candidate, plus two focused lookup controls for the later correction. Counts are
selected invocations, not deduplicated tests or performance measurements.
Both Linux toolchains passed check and strict clippy after the lookup correction.
Formatting, nine harness controls, doc-check, the registry (188 discovered gates, all
registered) and the local nonpromotable performance contract passed. The new
manual original-service gate is registered separately and is not run by CI.

The archive under local-runs/bootstrap-original-service-20261023 retains raw
check/native logs and checked source snapshots in a deduplicated SHA-256 object
pool. seal.py verifies exact objects and refuses overwriting the first archive.
Earlier missing-source, compatibility-view and source-drift refusals remain
negative. Binaries are represented by their receipts, not copied into Git.
The packet contains 2,815 logical files in 536 unique objects: fourteen check
lanes and two native lanes, including six negative lanes. Its index SHA-256 is
d3858133295f5d202be21c9dcce48d87051f5690d84a110ee131796f80059f82.
git-index-audit.json confirms every staged object and the index retain their exact
SHA-256 bytes. Only the opaque object pool disables text/whitespace processing;
no raw failure log or ordinary source style rule was rewritten to make it pass.

The temporarily installed diagnostic image was 1,680,168 bytes, SHA-256
4750a12568c4293ad8db6ce356abb272e36ea2eb81d6c1639c55f1a74947dfce.
The original-test libtest SHA-256 was
c7785f859cfc085ebfc9e8ba9dce93d5f036b46338f9adc7d5ebccda213946c0.
The frozen qualification manifest remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

## Next gate

The corrected non-loading preflight needs a separately recorded bounded original
attempt. The current runner's consumed output path prevents accidental replay;
do not delete or reuse it. Only after actual delivery and authenticated readiness
may the new path advance to original all-writer cessation and its SIGKILL/lost-ACK
matrix. Authenticated worker start, completion/cells 2–4, production enrollment,
connection recovery and server rehearsal remain distinct gates. This work adds
no throughput/allocation claims and opens no release installation or workload.

Implementation commits: d94217a1 (plan), 760ec76e (fixture and source contracts),
2a93ba34 (runner), a493b3ad (writable receipt fsync), 25af3c71 (Debian preparation
and persistent recovery), a2cc7b46 (non-loading lookup), d1879485 (exact evidence
sealer), 04cf0753 (opaque evidence attributes), 9802543a (retained packet).
The executed original candidate is 25af3c71, not the later lookup fix.
