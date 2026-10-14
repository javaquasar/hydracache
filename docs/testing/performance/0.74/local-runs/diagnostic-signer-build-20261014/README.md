# Local separate signer build model evidence

This packet records synthetic signature/content checks for the separate
signer-build receipt model. It does not enroll a production issuer or prove an
actual build, original executable/runtime custody, token exchange or bootstrap.
The [preregistration](../../diagnostic-signer-build-plan-20261014.md) precedes
implementation. Exact baseline, preregistration and tested-source commits,
source/unchanged hashes and raw capture digests live in [manifest.json](manifest.json).

## Tests and command outcomes

The Windows baseline passed five producer, 15 artifact/builder and eight
reviewed-trust cases. A mistaken `--test diagnostic_builder` invocation refused
with exit 101 because that target does not exist; the corrected existing
`diagnostic_artifacts` target supplies builder coverage. New integration tests
then refused at the missing module with E0432/exit 101 before implementation.

Nine signer-model cases pass on Windows and in three Linux repeats. They include
every statement field, all 128 signature positions, known old key-purpose
refusals, wrong key/domain, malformed schemas/JSON/encoding/bounds and all five
artifact substitutions. Seed `0x07403b22` checks 64 content mutations and four
independent concurrent models. The positive intentionally uses non-ELF bytes
and opaque contexts; only their selected digest/length and signature match.

Linux regression passes 12 producer, 29 artifact, eight reviewed-trust and six
kernel-binding API cases, plus 18 OIDC cases and 91 doctests. The first OIDC
filter `diagnostic_oidc` selected zero cases; this is retained, not counted as
coverage. The corrected `oidc` invocation records the actual 18-case regression.
Windows repeats the 15/eight old reader cases. Rust 1.94 package check/strict
Clippy and Rust 1.88 MSRV check run on both platforms. Root contracts cover six
gate-registry, 134 performance-contract and 13 release-evidence cases, with xtask
check/strict Clippy, formatting and local documentation/registry checks.
All 25 Python builder/public-source/acquisition regression fixtures also pass
without fetching GitHub. The packet's six copied-evidence tests pass on Windows
and Linux; their first verifier captures covered the initial 21 raw build/gate
logs, before the packet-test/format/Python captures were added to the manifest.

`performance-contract-check --release 0.74` is local/non-promotable. Its
`--require-ship` counterpart retains expected exit 1: candidate identity and
release qualification remain incomplete. These are correctness checks, not
product measurements, SIGKILL rehearsals or the full release milestone gate.

## Read only packet verification

Run `python verify_packet.py` from this directory. It compares current LF source
with the committed tested source, unchanged files with baseline, frozen observer
locks with their original commits, raw qualification/capture hashes and explicit
false admission claims. The verifier reads local Git and files only. A future
checkout changing a bound source must verify this packet at its retained source,
not rewrite old evidence. `python -m unittest test_verify_packet.py` exercises
control and copied-packet mutations without altering originals.

## Remaining execution boundary

The opaque result exposes only receipt inventory. The controller producer,
CLI, observer reader, reviewed pins, server route, locks, workflow and frozen
qualification inputs stay unchanged. Source IDs, clean flags, log, runtime
manifest and bootstrap context remain supplied assertions, not compilation or
job custody facts. The separate schema has no production receipt producer,
default root, trust installer or bootstrap consumer.

Independent signer issuer/pin selection and original prebuilt-artifact/runtime
custody must precede granting signing/token authority. Retained-body credential
coordination, original receiver/replay/common fence/start, all-writer cessation
and later fixed cells remain before any separately admitted server/pilot or
qualification run. No actual key/token, host/service action or workload follows
from this packet.
