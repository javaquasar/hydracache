# Collector Starting byte record, not publication

Source `8d1b5e2a6921f23ba0ba1d03af7e863f1aa1be25`, tree
`426f69d8d26020e65da145e7966e806cf6ce7d6d`, follows the
[preregistered byte slice](../../diagnostic-collector-starting-record-plan-20261016.md).
This local packet adds a strict encoder/inspector, not a new storage phase.
The existing Reserved inventory still accepts exactly its original two files.
No file is published, consent consumed, owner recovered or process started.

The private typed record binds the complete canonical controller-attempt and
Reserved LF records, the canonical signed-consent LF envelope and the identical
Starting state. The existing prior-history validator and adjacent transition
validator are reused. The record limit is 16384 bytes including LF; the original
consent limit remains 8192. The original consent's Reserved-state projection is
hashed without LF; the older Reserved document's projection uses LF. A negative
case checks that these distinct contracts cannot be conflated.

The inspector checks grammar and consistency, not cryptographic authority.
One explicit case accepts a signature-shaped zero value as byte evidence while
actual signature inspection rejects it. Neither signed time nor caller bytes
are a fresh kernel observation. The future publisher must retain the original
admitted chain and repeat its actual peer/time/trust/epoch/storage checks.

## Observed checks

Both Rust 1.94 and 1.88 pass five clock-free unit tests, one Linux public API
test and two compile-fail examples. The 3038-byte golden record refuses all
3038 strict prefixes. Seed `0x074057a1` orders 58 leaf mutations, nine rehashed
binding mutations and seven rehashed grammar/state mutations. Prior-history
substitution, unknown/duplicate/future/oversized/noncanonical records and the
data-versus-authority boundary are covered.

The unchanged Linux regressions pass 62 tests per compiler: IPC 24, lease 20,
original fence five, consent seven and host six. All 184 static contracts pass;
the registry discovers 177 gates, all registered. Affected checks and strict
Clippy pass on Windows/Linux and both compilers, as do formatting, documentation
and the nonpromotable local performance contract. No full milestone verify ran.
The ship guard correctly remains closed on identity/qualification/full-verify
requirements. This is not a release or performance claim.

The 38 command captures retain early failures: missing API, initial unit-test
assembly, historical source restoration, formatting order and missing registry
entry. The focused historical-source failure capture is explicitly truncated;
it is not an exact complete process stream. Final corrected-source gates are
separate captures. No failed runtime chain was retried into a green claim.

## Evidence verification

Run `powershell -NoProfile -File <packet>/verify.ps1` from any directory.
The verifier pins the committed eleven-file delta, tree, ancestry, 22 unchanged
runtime/qualification blobs, all 38 command captures and coverage/scope fields.
Optional `-CheckWorkingCopy` also checks the eleven recorded checkout byte
hashes; default verification remains useful after later source changes.
External canaries promoting server readiness or reducing leaf coverage refuse;
[verification-results.json](verification-results.json) retains the observations.
This unsigned evidence is not an attestation or a signature-verifying inspector.

The separate [server frame packet](../validation-frame-server-fault-20261016/README.md)
closes only the previous frame fault gate. This packet runs no server command.
Next: consuming private storage phase, exclusive write/seal/file-and-directory
fsync, uncertain acknowledgement and real SIGKILL/lost-ack boundaries. Original
start, all-writer cessation, later cells, signed installation and server
admission remain closed; no product workload is authorized by this result.
