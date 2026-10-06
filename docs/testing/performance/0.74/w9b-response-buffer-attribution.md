# W9b: canonical large-response allocation attribution

## Status and scope

This is a completed local D1 owner screen, not an accepted product candidate. It
does not reopen the negative W3 batching decisions or W4's small-payload allocation
assessment. Product runtime and native/shared/embedded paths are unchanged;
C74 remains `UNRESOLVED`, accepted product proposals remain zero, and admission is
closed. No rented host or qualification workload was used.

The preregistration is `w9b-response-buffer-attribution-contract.toml`. Clean
source `cc0b2fbfc0b7d9c4c9f37e96089f37134fdf546f` was built in release mode from
baseline `7919e6d7887316bbd6752d945fde240aa9fdea17`. Its profiler binary SHA-256 is
`d1b04c9f22996e95b7e7ac11daff97c713f48a19fc9fba60070601778b249ad3`.
Profile identity changed to `w4-w5-w9b-resp-stage-profile-074-v3`: earlier v2
receipts are not reinterpreted as this measurement.

## Hypothesis and matched controls

Large GET replies might make repeated allocation of the encoded output material.
The proposed follow-up would reuse one serial encoding scratch buffer, not combine
responses or move the next command before the preceding write and flush. The
existing `Vec::from(BytesMut)` transfers ownership; the hypothesis is avoiding the
fresh output allocation, not removing an invented extra copy at that conversion.

The profiler now measures two independently initialized `RedisRespServer` states:
canonical `execute_command`, and identical execution followed by the existing
`encode_resp2_value`. Both use namespace `default`, the same binary-safe key,
payload, request-ID sequence, one preload and 100 warmup commands. Each measured
pass validates exact results without constructing expected payloads in its
allocation window. The encoded control also validates header, payload and CRLF.
Both allocation and separate timing passes have N operations and checksum N.
Post-window checks require exact dispatch/mutation totals, one final store entry
and the final stored value. After final verification both controls have `2*N+102`
dispatches; mutations before that read are 1 for GET and `2*N+101` for SET.

This denominator includes command construction, canonical dispatch/store/reduction
and encoding. It excludes decoding, socket IO and connection scheduling. It must
not be divided into an unrelated network benchmark's total or called complete
end-to-end allocation. CPU fields include counting-allocator overhead and Windows
CPU granularity; they are preserved but not used for product performance claims.

## Execution and repeatable results

Exactly 15 fresh processes ran: three repetitions of five preregistered cells,
rotating the cell order once per repetition. Seed is 740074, concurrency is one,
GET hit ratio is 100%. GET/SET at 256 and 4096 bytes use 10,000 operations per
window; GET at 1 MiB uses 500. No failed benchmark attempt, retry, best-sample
selection or product candidate exists. All raw receipts are retained.

| Operation / payload | Execution B/op | Execution + encode B/op | Encoding increment B/op | Share of this measured response path |
| --- | ---: | ---: | ---: | ---: |
| GET / 256 B | 1,291.602 | 1,555.602 | 264 | 16.97% |
| SET / 256 B | 1,291.602 | 1,299.602 | 8 | 0.616% |
| GET / 4 KiB | 8,971.602 | 13,076.602 | 4,105 | 31.39% |
| SET / 4 KiB | 8,971.602 | 8,979.602 | 8 | 0.0891% |
| GET / 1 MiB | 2,097,927 | 3,146,515.168 | 1,048,588.168 | 33.325% |

Allocation totals, differences and workload/request/key/payload digests match
within all three repeats of every cell. The independent isolated encoder control
matches the canonical subtraction exactly except at 1 MiB: isolated encoding is
1,048,588 B/op, leaving an 84-byte residual over each 500-operation canonical
window (0.168 B/op). This residual is retained as unassigned control overhead,
not claimed to belong to the encoder. It is not removed by sample selection.

The consolidated record is
`local-runs/w9b-serial-encoder-owner-cc0b2fbf.json`; its 15 referenced raw receipts
include original tool-output hashes. Repository text import normalizes LF and
adds a terminal LF only. A regression verifies hashes after undoing this explicit
normalization, all registered cells/repeats, source/binary/profile/workload
identities, dispatch counts, checksums and deterministic allocation totals.

## Verification and development diagnostics

The profiler passes six tests, including malformed-result rejection, binary keys,
large payloads, exact state reconciliation and allocation-free validators. The
initial parallel validator test charged 75 sibling-test bytes and failed; a
filtered child process now enforces the same zero-byte budget without concurrent
tests. No budget was increased. A mistyped xtask selector was corrected and the
actual evidence target executed; the invalid invocation counted as no test proof.

Source-bound checks passed 39 performance-contract, 13 evidence, 23 governance
and 16 doc-check tests, affected all-target checking and strict all-target,
all-feature lint, format, the local non-promotable contract and W11/W12 expected-red
canaries. The additional archived-receipt regression is verified in the evidence
commit. This is not full-workspace or new hosted-CI qualification.

Reproduction from the exact source (repeat for the registered matrix):

```powershell
cargo build --release --manifest-path tools/resp-stage-profile-074/Cargo.toml --locked
tools/resp-stage-profile-074/target/release/resp-stage-profile-074.exe --source-commit cc0b2fbfc0b7d9c4c9f37e96089f37134fdf546f --operation get --payload-bytes 4096 --iterations 10000 --seed 740074 --output target/performance-evidence/0.74/w9b-reproduction-get4096.json
cargo test --manifest-path tools/resp-stage-profile-074/Cargo.toml --locked
cargo test -p xtask --test performance_contract_074 --locked
```

## Decision and next isolated step

Large GET has a measured encoding owner worth a D2 review. Small SET does not;
its eight-byte output is not a reason to rewrite shared/native storage. The data
does not prove the unchanged 20% affected end-to-end allocation floor or any
goodput, latency, CPU, RSS, retained-memory, copy-byte or syscall improvement.
Pipeline-one must keep the canonical path; scratch must be released before
waiting for another read, with no larger output/input/idle-retention bound and no
cross-connection pool. Each command must still execute after the previous complete
write and flush, and its real response must follow execution.

D2 must first test this bounded lifetime, fragmented pipelines, partial writes,
disconnects, slow readers, large/mixed replies, errors and RESP3, preserving the
existing native interleaving/expiration/quota guards. Only a semantically admitted
single-hypothesis candidate proceeds to at least five counterbalanced independent
D3 pairs with separate RESP, native, ClientSurfaceState and embedded controls.
Unaffected goodput must remain at least 0.98, CPU/p99 at most 1.03, and affected
end-to-end allocation reduction at least 0.20. No threshold was changed. Idle
retention and allocator/copy tradeoffs remain risks; scratch reuse cannot be
presented as batching, syscall reduction or elimination of payload copying.
