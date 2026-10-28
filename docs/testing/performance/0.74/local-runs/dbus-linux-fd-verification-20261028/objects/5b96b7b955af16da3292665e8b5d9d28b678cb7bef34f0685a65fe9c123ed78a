# GNU D-Bus, actual descriptor transfer and raw-FD CLOEXEC

The [selected read-flow shadow](dbus-read-flows-result-20261028.md) now has GNU/Linux
coverage and private Unix socketpair/SCM_RIGHTS controls. A stronger test finds a
real missing CLOEXEC flag on the raw descriptor held by a received message. A
separate pinned Linux recvmsg leaf patch fixes that candidate; its unchanged full
catalog passes 38 tests on Rust 1.94 and 1.88, plus check and strict Clippy.

This remains a standalone dependency/runtime candidate. The root graph and
supervisor, authority policy, live checks and original windows are unchanged.
No release installation, system/session bus, systemd operation, clock change,
server, product load or qualification occurs. No performance claim follows.

## Exact source and local execution

The producer verifies the preceding packet's independent SHA, source/overlay
identities, original supervisor source pins and protected root files. It copies
both source views to fresh nonroot private ext4 directories under
/home/artur/.cache/hydracache-074-dbus-linux. Only test include-path spelling
changes from Windows to those owned asset paths. The Unix module is a separate
test addition. The same 62 lock identities, 58 primary registry archive checksums
and three selected zbus features remain. Each private CARGO_HOME uses a directory
source replacement populated from those exact archives, with generated Cargo
file checksums. All Cargo commands are offline and locked; no toolchain, package
or system configuration is installed or updated.

Actual GNU Cargo compiler-artifact paths, executable SHA/size and complete lists
precede native execution. Each final source/compiler view has its own target
directory. Native exits, identities, complete counts and ignored/filtered totals
must agree. The original 31-case source baseline passes on both 1.94 and 1.88,
including check and strict Clippy, before any accepted receive-path change.

The fixed Linux workers reuse the independently pinned CI LinuxGroup and owned
envelope mechanism: exclusive child reaping, retained pidfds and kernel ECHILD
retirement, without numeric PGID signalling. Inner session origins are capped
at 1100 seconds within the unchanged outer 1200-second local envelope. Physical
and native 4 GiB floors, five-second capacity polls and five-second retirement
remain. These are CI containment receipts, not production watchdog or all-writer
authority. Completed/failed attempts retain their separate first records and
boot identities; they are not combined into a mixed-lifetime qualification.

## First outcomes retained

The first GNU session passes the complete 31-case baseline on both compilers.
Its first new FD build then refuses E0599 for build_message_stream, followed by
two cascading E0282 errors. That API requires the unselected bus-impl feature.
The corrected fixture uses ordinary build and arms its stream before returning
the private p2p pair and before any user request. It adds no feature and changes
no caller, dependency, deadline or validation predicate.

The second dispatch contains a coordinator selector error: it invokes the old
native context. Exclusive directory creation refuses the existing first root;
exclusive receipt publication also refuses overwrite. No new Cargo or native
test phase begins. Both streams and the exact first/second native producers
are retained. Enum-based argv/context controls precede the corrected third
dispatch. That dispatch passes the full 37-case FD catalog, check and strict
Clippy on both compilers.

Preparation of a fourth, stronger oracle also catches a raw-prefix mismatch:
Rustfmt selected LF for a new file while its passing predecessor had CRLF. The
first inert byte refusal and LF candidate remain; a distinct corrected prepare
restores the exact prior byte prefix. It does not normalize the validator's
expected source, modify the first result or dispatch Linux before that guard.

## Descriptor controls and the missing observation

The six initial Unix tests exercise:

- EXTERNAL authentication over a real socketpair and actual kernel peer UID/PID;
- FD request/reply roundtrip, exact dev/inode and a shared open-file position;
- CLOEXEC on both *decoded* owned descriptor copies;
- a 262144-byte payload plus its descriptor, with exact returned bytes;
- disconnect after receipt and an effect written before an acknowledgement is
  lost, each with exactly one request and no automatic reissue;
- rejection of a wrong response type, then explicit connection retirement and
  a per-inode descriptor census leaving only the original fixture descriptor.

The initial 37/37 outcome is valid for those predicates. It does not establish
CLOEXEC on the message's own received FD or its release before closing the
connection. The added seventh test holds the actual reply, asserts its raw FD
count/identity/CLOEXEC, and introduces a later response as a causal reader
barrier. It then rejects String decoding, drops the held message, requires only
the original descriptor to remain, and sends another request successfully
*before* closing the connection. It uses no sleep, polling or retry-to-pass.

That first 38-case command completes **37 passed / 1 failed** on Rust 1.94.
Only received_fd_drops_before_live_connection_close fails, at the raw kernel
CLOEXEC assertion: the masked flag is zero. The later lifetime/barrier assertions
are not reached in that first failing case. The candidate stops there; MSRV and
later stages are not reported as executed. Complete first failure/accounting,
source, streams and owned retirement remain immutable.

## One isolated receive-path patch

The pinned source is zbus/src/runtime/io/unix.rs, SHA-256
`f8cc21f18420cae201ba3faa43ea1e29d0fbf45af00c4938dcf5159c82907c76`.
Its fd_recvmsg calls recvmsg with RecvFlags::empty. Decoding an OwnedFd creates a
separate CLOEXEC duplicate, explaining why the earlier decoded-only checks pass
while the raw message descriptor remains unprotected. The finding concerns the
tested local zbus 6.1.0 source, not a demonstrated exposure in the unchanged
root product graph or a claim about every upstream version/platform.

The [reviewable leaf patch](../../../../tools/dbus-linux-074/linux-recvmsg-cloexec.patch)
sets RecvFlags::CMSG_CLOEXEC in that receive syscall on Linux. Other targets
retain their previous flags and are not qualified as fixed here. The kernel
sets close-on-exec on the SCM_RIGHTS descriptors during receipt; a later fcntl
would leave an intervening inheritance window. See the primary Linux
[recvmsg documentation](https://man7.org/linux/man-pages/man2/recvmsg.2.html).
The source guard accepts one exact preimage and one exact call; the inverse
restores the original bytes. The rest of the receive function, FD limit, ancillary
handling, error branches and complete caller/live frame remain unchanged. No
second receive-path hypothesis is bundled into this candidate.

The fifth native context keeps the stronger test body exact and changes only
this leaf in the standalone runtime. **38/38 pass on Rust 1.94 and 38/38 on 1.88**;
both all-target/all-feature probe checks and strict Clippy pass. The held raw
descriptor now has CLOEXEC; the same inode descriptor is released after refusal
while another request still succeeds on the live connection. Per-inode alias
checks and owned fixture cleanup pass. No version, lock, feature, deny exception,
MSRV or original timeout is changed to obtain acceptance.

## Checks and retained artifacts

Seven Python controls cover exact 31/37/38 catalogs, missing/foreign/duplicate
identities and false summaries, all five explicit argv/context pairs, unknown
selectors, and the pinned patch/inverse with stale-source refusal. Owned Windows
controls and Rust formatting pass, including the actual patched receive source.
Native and Windows containment receipts retire without errors. The final native
session also verifies all vendor file hashes and absence of owned fixture files.
No fresh product package/workspace or GNU cargo-deny command is claimed; the
root policy and graph remain exact and their separate admission gates remain.

The source-only commit is `bae218bb05d80c068369dc9f63d223cbaaa4d2d5`.
The [primary packet](local-runs/dbus-linux-fd-20261028/index.json) contains 3300
logical references and 517 distinct raw objects. Its index SHA-256 is
`9de2f42856957da48aac3f87f1e8a1d00a6e7f2d259c2f36b978c0a06a4d5ce4`.
The patched leaf SHA-256 is
`a486d9b0ab4d0d245f82da823d02a6fd750efe0e0f8dda6b1b5c078bca282151`.
It retains all five source/input contexts, exact first producers, preparation
refusals, raw command streams, Cargo artifact/catalog/accounting and ownership
receipts, source/leaf inverse proof, all 58 primary archives and vendor maps.
Native binary/target trees remain outside the packet. Inert verification compares
source/trace/checksum bytes without executing archived code. All old protocol
traces agree across Windows and the seven GNU image outcomes, including the
first stronger-test failure; final Unix trace markers agree between compilers.

## Remaining boundary

These tests use same-process private socketpairs, not the system manager or an
enrolled original owner. Real UID/PID in this fixture do not bind a production
manager, launcher, executable, streams, cgroup, ReplyState or policy epoch. The
lost acknowledgement is a small nondurable fixture effect, not durable lifecycle
completion, SIGKILL recovery or exactly-once production execution. CLOEXEC concerns
exec inheritance; it does not revoke existing writer aliases or prove cessation
of all writers. Large payload success does not prove forced partial-write cuts.

Forced fragmented/partial I/O, ancillary truncation and disconnect boundaries,
actual original ReplyState/FD binding, remaining source/effect/distribution review
and fresh complete product package/workspace gates still precede dependency
adoption. Non-Linux targets and additional upstream features require their own
coverage. Authenticated original execution, autonomous writer retirement, durable
cells 2–4, reconnect, trust/enrollment and admitted-host rehearsal remain open.
