# Reuse of original worker policy verification

W11 tooling proposal against 78d8d9fc, attributed by the immutable
validation-cost-20261016 packet. This is one hypothesis: repeated parsing and
signature verification of the same admitted worker envelope can be replaced by
integrity checks on its already authenticated envelope and retained projection.
No HydraCache product/native/RESP candidate or server operation is admitted.

## Proof boundary

Keep the original constructor and full verifier byte-identical. A non-cloneable,
non-deserializable checked owner already holds full original projection, original
trust (including key/body pin/epoch), asserted context and signed-envelope SHA256.
Retain revalidate's refusal, current trust equality, context equality, size bound,
original-envelope SHA256 and sticky error order. Replace only the repeated full
verifier and projection equality with SHA256 of the retained projection's canonical
LF body compared to the original trusted body pin. This also detects deliberate
private projection corruption in existing tests. No extra retained field/buffer,
global cache, public mode/factory, lifetime extension or trust/epoch refresh.
The proof relies on the same SHA256 integrity assumptions as the existing pins.

All source/authority/kernel/file/configuration/peer/time brackets stay unchanged.
Current revocation, host/boot/namespace change, changed key or body pin, tamper,
oversize and already refused state must produce the same typed error and latch.
Preserve the exact old method as a cfg(test)-only full reference oracle. Compare
results and sticky recovery for fixed and seeded mutation sequences, including
internal retained-projection corruption and independent concurrent owners.

## Test and measurement sequence

First freshly prebuild baseline libtest and retain a separate hash-pinned copy
outside the checkout. Run its registered exact stage probe before mutation.
Before runtime implementation add an exact mechanism test: one full worker
verification at construction; none in repeated revalidation. Change only the
probe's implementation-specific positive-call assertion to the preregistered
exact expectation (Setup=1, other measured stages=0). Keep every action and
semantic/file/lock assertion byte-identical. Record its compiler/test red.
The old probe source remains in the baseline commit/binary and prior packet.
Update only the manual registry reason from no production optimization to no
product workload; keep command, tier, budgets and all old tests unchanged.

After semantic/reference gates are green, execute five independently started
current-toolchain pairs in B/C, C/B, B/C, C/B, B/C order, sequentially with no
concurrent builds or workspace formatting. Baseline is the retained original
binary, candidate is the fresh current binary; pin both SHA256 and selected
source/fixture/instrumentation identities. Do not retry or discard a failed pair.
For Publish and Bind, estimate exp(mean(log(candidate_ns / baseline_ns))) with
a paired percentile bootstrap, 10000 resamples, seed 0x0740c0a0, 95% interval.
The upper interval bound must be <=0.80 for both stages. Request authentication/
signature call counts must match exactly; only full worker verification counts
fall from baseline to the exact candidate expectation. This is local debug
tooling attribution, not CPU/op, release qualification or a native/RESP gain.
If this diagnostic effect is absent, retain the negative and revert only this
runtime substitution, not unrelated work or evidence. No thresholds are tuned.

Run current/MSRV semantic, original-input, kernel, signed-projection, affected
context/credential and negative API tests, plus the unchanged causal manual proof.
Run affected-package check/strict Clippy on Linux/Windows current/MSRV, static
contracts with exact approved source substitutions and guard canaries, format,
documentation/registry/local contract. Keep Time, Kernel(Invalid) and IPC Host(Busy)
historical causes unresolved unless this slice supplies occurrence-specific proof.
Starting, runtime handoff, writer cessation, production enrollment, server, expensive
infrastructure, qualification and push stay closed. Full verify is for the later
release milestone, not implied by focused checks here.
