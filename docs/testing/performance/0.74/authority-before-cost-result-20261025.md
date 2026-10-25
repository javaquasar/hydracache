# Authority before read validation cost

Fresh `fstat` projection now replaces only the two before-read file metadata
sites in `Leaf::read` and `Leaf::read_reusing` on Linux x86_64. Eight independent
processes support approximately 5.7% lower CPU cost for the complete leaf
validator with both marker and record payloads, against the frozen same-image
reference. The parent-device query, all four fresh observations on a complete
revalidation, original predicates, conditional order and deadlines remain intact.
The first full preparation still expires during enrollment; this is a local
control-path improvement, not server-start or product admission.

## Baseline and isolated hypothesis

The [directory candidate](authority-directory-cost-result-20261025.md) leaves
leaf before-read and parent-device metadata unchanged. Before-read is a separate
consumer: it needs all eleven Stamp fields for the later identity/content
comparison, and must first reject nonregular files, wrong UID/GID, wrong complete
permission bits or link count other than one. These Invalid refusals precede
the parent-device/size Budget branch. A projection that merely obtains identity
without these predicates would change the authority contract.

Commit `3da3c31201b8593dfd938140c4c069e7e05b30ed` freezes the
[protocol](../../../../tools/authority-before-cost-074/protocol.json), test-only
reference/prototype, reader and manual registry before measurement. Runtime
remains the directory candidate at this stage. The reference retains its fresh
held/after Stamp projections and original before-read/parent `Metadata` queries.
Only the prototype's before-read projection changes.

The first probe compiles and passes check/Clippy, but its new field test fails:
60 passed, 1 failed, 5 ignored. The fixture uses anonymous `tempfile()`, whose
link count is zero, while expecting a valid one-link file. A distinct corrected
source copy uses `NamedTempFile` and explicitly requires the original regular
predicate to accept the starting object. No expected refusal or production
check is relaxed; no CPU cohort uses the faulty image. The corrected probe
passes 61 tests with five manual screens ignored, image SHA-256
`3b8a864c6a41c70171e364f398744613e5786c1328bca17d7e9f0dfad12278cc`.

## Comparison and measured result

Each phase starts eight nonroot Rust 1.94 GNU release processes, pinning each
measurement child to CPU 5 without overlapping this session's builds or tests.
Each process measures a 65-byte marker with budget 65, then a 1024-byte record
with budget 2048. Four balanced arms execute frozen reference, identical-reference
control, actual runtime and prototype. Each arm has 32 warmups and sixteen
samples of 1024 full leaf validations. Eight forward/reverse cyclic orders run
twice, rotated by process. No samples or processes are discarded.

The estimator is the median over eight process medians of paired log CPU
ratios. Its 95% percentile interval uses 20000 process bootstrap resamples and
seed `0x074d1a05`. Identical-reference control must fit [0.97,1.03]; baseline
actual-runtime/reference must fit that interval before integration, and actual
runtime/prototype must fit it afterward. Candidate upper interval must be at
most 0.98 for both payloads, as frozen before observation.

| Full leaf payload | Baseline reference CPU per call | Prototype ratio and 95% interval | Integrated runtime ratio and 95% interval |
| --- | ---: | --- | --- |
| Marker | 5.979 microseconds | 0.944920 [0.943081,0.945445] | 0.942904 [0.942236,0.943886] |
| Record | 6.001 microseconds | 0.944638 [0.944047,0.945670] | 0.942807 [0.942318,0.943157] |

All controls and both phases pass. Baseline runtime/reference ratios are
0.999380 and 1.000128. Integrated runtime/prototype ratios are 0.999129 and
0.998157. These comparisons cover the complete leaf method, including open,
four metadata observations, bounded read, byte/stamp comparison and scratch
handling. They do not measure complete registry/preparation or native/RESP
requests. Ratios from earlier nested experiments must not be added to these.

One local renderer assertion refuses before SSH: the build label appears twice,
but the renderer expects one occurrence. Correcting that occurrence assertion
does not dispatch an extra remote measurement. The refusal is retained with
`remote_dispatched=false`; all sixteen remote CPU children are distinct.

## Runtime integration and fault controls

Runtime commit `bac965b92a64d1f042fa6094b261dcdb614407da` adds
`Stamp::fresh_regular` and its checked predicate in `diagnostic_authority_stamp.rs`.
Linux x86_64 uses the existing fresh eleven-field `fstat` projection. Other Linux
architectures use the original `File::metadata` and `regular` predicate. The two
call sites in `diagnostic_authority_fixture.rs` consume that Stamp; parent-device
metadata still executes at the same conditional site before the size check.
No publication, signature, clock, durable/wire artifact, ownership or authority
capability changes.

The first new test checks original/prototype/runtime type, owner, group,
permissions and link predicates; it distinguishes those fields from benign
identity/size/time changes in the regular predicate itself. The complete Stamp
comparison still observes those latter changes. The second exercises fourteen
full leaf scenarios: healthy and empty payloads, UID/GID mismatch, budget,
permissions/setuid, extra link, oversize, missing/replaced/symlink name, NUL and
held identity failure. Test-only restamping exposes before-read refusals instead
of letting the earlier held comparison mask every case. Invalid must still
precede Budget; held mismatch must still suppress opening a missing name.
Results, errno and scratch retention/eviction agree across all three paths.

Native release build, all-target/all-feature check, strict Clippy and 61
authority cases pass; five manual screens remain ignored in that normal run.
The runtime image SHA-256 is
`92fb7f268cf08b86b5989b8c21460e8f7feab5c600fc5df6046340dd4814df10`.
Both private source manifests contain 2564 inputs. Reversing only the two
before-read edits, new helper/import and three added runtime assertions exactly
reconstructs the frozen baseline after line-ending normalization. The benchmark,
reference, prototype and thresholds are unchanged during integration.

## First complete preparation and next boundary

The first full five-case holder attempt uses this exact runtime image, the same
nine-file bundle, original affinity, 1200-second outer bound and 300/60/10-second
windows, without case/cost/material overrides. The healthy child and parent exit
101 at `consuming_held_live.rs:267`, `enroll_once`, with nested `Causal(Time)`.
First expiry age is 10100156571 ns: issued BOOTTIME 1655423526468721 ns,
observed 1655433626625292 ns, same boot. Zero cases complete; the four later
cases remain unexecuted and this candidate is not retried into a pass.

One failed frontier cannot quantify an end-to-end improvement or establish
original delivery, every-writer retirement, completion or later cells. The next
isolated hypothesis is the remaining parent-device metadata query; Boot/Namespace
costs remain a separate larger attribution target. Neither permits reducing the
number of live observations or extending a window.

The [evidence index](local-runs/authority-before-cost-20261025/index.json) has
313 members/225 objects, SHA-256
`4136e44c8a3adf1c06c234861406b730ebd99644c2bd3af48b36931e3e2c6466`.
It retains first faulty/corrected/runtime builds, both raw cohorts, exact
commands/manifests, source proof, local checks, host frames and first full
refusal. Six reader controls and four saved-byte audit tests pass. Independent
staged-blob review reproduces both estimates and refuses promoted failures,
changed windows, missing lint flags, process reuse and unrelated source edits.

The [final checks packet](local-runs/authority-before-checks-20261025/index.json)
retains the staged verifier and successful doc-check/local non-promotable
performance-contract logs: fourteen members/thirteen objects, index SHA-256
`b165433d16b5e6398935f0d2eb4a3dcce80c0edaf0ddfb4645a498b7e60d3c9d`.

Endpoint frames preserve boot, PID1 mount table, nine absent campaign/lifecycle
markers, installed binary/receipt hashes and supervisor PID 8839/NRestarts 0.
No fixture cgroup remains. AppArmor, clocks, services, installed artifacts and
dependencies are unchanged; no product workload executes. Manager 255 delivery
compatibility, retained signer/AppArmor refusals and the red full preparation
remain admission blockers. The qualification manifest is still
`prepared-not-authorized`, expensive runs disabled, SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
