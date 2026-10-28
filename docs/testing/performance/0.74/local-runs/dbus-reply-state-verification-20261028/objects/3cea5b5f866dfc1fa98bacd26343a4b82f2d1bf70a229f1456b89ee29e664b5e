# Original ReplyState compatibility with the standalone zbus backport

The held GNU zbus 6 candidate now passes the selected **25 original ReplyState
cases**, alongside all **42 predecessor cases**, on Rust **1.94 and 1.88**.
Both compilers also pass all-target/all-feature probe check and strict Clippy.
This closes a local correlation/ownership compatibility gate. It does not adopt
zbus into the root graph, authenticate a manager, start an original worker,
prove all-writer cessation, or admit a release/server run.

## Exact source and adaptations

The projection pins the complete existing diagnostic_dispatch_exchange.rs
(SHA-256 `3a10d18ef079e33da17e7e48dc28b34e43c11fcc315985bdc91f82d3bcd1ddbf`)
and four existing test files. Execution logic changes only zvariant → wire
namespace and the NoReplyExpected bitflag spelling; every request/reply predicate,
order, error branch and state transition stays exact. The copied 65,536-byte
protocol constant has a separate whole-source pin and supplies no authority.

Fixture-only changes remove eager builder-result unwraps, because zbus 6 records
conversion errors until build; final build unwraps and every assertion remain.
Value is imported as `Variant as Value`, matching the upstream compatibility
alias's underlying type. No deprecated alias or lint suppression is used.
Transport-specific socket/negotiated submodules are excluded explicitly; no live
owner/manager validation is replaced with a successful stub. Existing private
transport/fragment controls remain in the predecessor 42-case catalog.

The fixture adds direct libc 0.2.191 and tempfile 3.27.0 edges already present in
the held graph. All 62 package identities/checksums and non-probe lock rows remain
exact. The offline input still contains 58 primary registry archives. No root
Cargo.toml/Cargo.lock, deny exception, supervisor or qualification identity changes.
The parent CI owner is independently pinned; the added import/projection controls
are not a product source/effect approval.

## What the original corpus proves

The request corpus rejects malformed manager/client claims, wrong paths/members/
interfaces, sender/reply fields, all request flags, oversized or wrong bodies,
missing/extra/aliased/read-side/nonblocking/non-CLOEXEC descriptors. Two original
message-owned pipe writers must pass fresh fstat/fcntl observations and differ.
The reply corpus preserves serial/sender/destination/header checks, the exact
MethodReturn flag allowance, canonical positive u32 job paths and manager-error
classification without exposing the error body. Malformed, early, duplicate,
delayed-after-lost-ACK and repeated-arm paths remain poisoned rather than retryable.

Borrowed ReplyState/OriginalReplyState keep no message or descriptor authority.
The owning delegate retains its original descriptors; a hidden Message clone can
keep writers alive even after JobObserved/drop. That last negative control is
important: a correlated job path is not execution admission or proof of retirement.
OriginalReplyState also rechecks names/serial/prepared-closed state and current
writer shape, including a descriptor whose CLOEXEC bit changed after preparation.

The unchanged independent model uses seed `0740e518`, 512 sequences and 16,384
random steps plus fixed prefixes: **17,150 checks per compiler**. Exact result
coverage is `[360, 1714, 117, 1636, 13265, 58]`; both owning and borrowed states
match the model at every action. These are deterministic coverage counts, not
performance measurements. Prior caller/read/peer-mismatch traces are byte-value
identical after decoding, with no extra requests or changed read order.

## First outcomes and containment

The first native build stops before tests with E0432: a mechanical namespace
replacement incorrectly imports wire::Value. Its complete command/streams/source/
retirement receipts remain. A separate exclusive candidate imports Variant under
the local Value name and passes the unchanged predicates and assertions; the
failed context is not overwritten or rerun. Initial inert Windows path-separator
and nullable compiler-diagnostic selector errors are recorded as operator notes,
explicitly distinguished from raw captured native streams. The corrected eight
inert controls pass. The exploratory formatting check across the five unchanged original files
refuses one pre-existing single-line println in the seeded-model test. The exact
old test source is intentionally preserved, not silently reformatted or declared
passing. A separately scoped formatting check covers the projected execution
module; Python controls cover the new adapter. This inherited full-format debt
and the documentation check are recorded in the verification packet. The full
root package/workspace gate remains open.

The original local 1200-second outer/1100-second inner absolute origins, 4 GiB
physical/native floors, five-second polling/retirement and predecessor 250-ms
method/frame guards remain. The same pinned Windows Job and Linux subreaper/
pidfd/ECHILD owners supervise exact compiler images and complete test catalogs.
No clock, service, system/session bus, installation, server, product load or
qualification is invoked. No performance win is claimed.

## Evidence and next boundary

The [primary packet](local-runs/dbus-reply-state-20261028/index.json) retains
1,344 logical members / 427 raw objects, exact input and projection diffs, primary
archives, first refusal and both final catalogs/accounting. Index SHA-256:
`59d8018a968bf93e59073bf15791d17b861d3bdd7751821d45342c0df52c0072`.
Native binaries/build trees are excluded. Inert verification rechecks every raw
object, exact projection, full catalogs, first refusal and both model transcripts.

The user has now approved preparation of maintained Chitchat/Zstd and zbus
backport packages under Rust 1.88, with no publication/installation/server run.
The [approved boundary and remaining package work](remaining-security-feasibility-20261028.md)
remain explicit. Next: finish standalone zbus migration/source/effect/distribution
coverage and the Chitchat wide-wire/Clippy/backport-package gates, then review the
exact final graph before root adoption and a new full package/workspace milestone.
Original execution, watchdog/all-writer retirement, completion/cells 2–4,
SIGKILL/lost-ACK and reconnect gates remain open; this packet does not substitute
for any of them.
