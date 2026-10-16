# Original pre-consent constant-depth semantic gate

Source `77b3ad15c8f56cd1ef41c02e31c1bad622126a55`, approved design
`c908017d`; baseline `ce4a0f54`. No publisher is present in this source.

Both normal GNU release test binaries passed the same 48-case nonroot server
matrix: 35 working-cut drift cases, five independent old-path reference refusals,
three invalid consent cases, two invalid request IDs, one already-refused
constructor, one healthy same-receiver control and one concurrent closing-round
mutation. Seeded 3072 clock edges, six compile-fail examples, the positive named
owner API, 62 regressions per compiler, strict Clippy/check on Windows and Linux
(Rust 1.94/1.88), formatting, 185 static contracts and registry178 passed.

All original guards and exclusive time origins remain live. The new preparation
freezes exactly the old projection once. Consent acceptance enters the existing
two-round full consent frame directly. No signature proof is cached, no deadline
is renewed, and no acknowledgement or execution authority comes from retained
bytes.

`matrix-*.log` are raw SCP copies. JSON command observations preserve the missing
API red baseline, the initial formatting-only source-comparison refusal and the
early launch before SCP completed (no test ran). A Windows relink attempt also
encountered LINK1104 while the preceding contract executable was still running;
that orchestration observation was not separately retained as a raw log. The
final contract run waited for its predecessor and passed. None of these failures
is counted as runtime success or erased by retrying a failed server scenario.

Temporary test root: `/tmp/hc074-preconsent-20261016.m9VZny`. Test helpers also
own separate `/tmp/hydracache-authority-*` leaves required by their existing
grammar. Stable supervisor PID8839/restarts0, boot, installed binary/receipt and
absence of production contexts were checked before/between/after the two lanes.
No installation, service stop, production lock, clock adjustment or workload.
Unsigned prebuilt test custody is recorded, not a signed production receipt.

Run `powershell -NoProfile -File ./verify.ps1` from this directory. The verifier
uses historical Git blobs, so later publication work does not rewrite this proof.
This packet proves semantic composition only; the separately restored publisher
must still pass its original complete fault/SIGKILL/lost-ack gate.
