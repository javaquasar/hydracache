# Pure sample boot ownership baseline

## Scope

Starting tree: `79dcf64bee22d801dc983ef62b4c2fefa44a0988`. Isolate the owned
boot clone in Origin::sample without changing representation, runtime or the
surrounding validation frame. Keep the complete sample body, KernelTimePoint
declaration/getters, and boot decoder in a source-checked synthetic model.
Both revalidations and both clock calls remain, in their original order.
Receivers are synthetic; there is no real Origin, clock or kernel observation.

Reuse the exact isolated allocator from the previous proc document baseline.
Use only installed Windows MSVC Rust 1.94.0 and 1.88.0, offline/locked. Do not
run WSL, an ignored test, a live profile, a server action or product workload.

## Fixed cohort and controls

Nine cases cover empty, lengths 1/35/36/37/64/1024, a valid 36-byte boot with
spare input capacity, and a generic 36-byte UTF-8 value. The ordinary 36-byte
case and spare-capacity case use the same valid boot UUID. Generic cases test
clone compatibility, never admission. Use 256 sample calls per case. Preserve
each case separately; do not turn this synthetic weighting into a production
distribution or pool stage controls into it.

Measure Rust allocator requests/deallocations only. Separate one-case controls
compare the unchanged prelude, the clone, drop and full sample. Verify zero
requests before the closing check and one owned clone on nonempty success.
Exercise all 16 masks of the four fallible calls and their first-error prefixes;
every refused path must return before the clone. This is a model property, not
real kernel failure or writer-cessation evidence.

Use seed `0x074b0a19` for 256 valid boot payloads, independent ownership after
Origin mutation/drop, exact getters, and independent point clones. Check the
unchanged boot decoder's bounded count, lowercase hex, hyphens and newline.
No production frequency, CPU/time, RSS/retention or admission-headroom claim is
accepted. An inline/shared boot representation is a separate candidate decision.

## Gate and evidence

Require exact source/mutation/capability canaries, seven pure tests per compiler
including the two unchanged allocator controls, check/strict clippy, direct
formatting, offline reviewer mutations and structural gates. Preserve each first
outcome; any repair needs a new cohort, never replacement or retry-to-pass.
Verify fourteen preceding raw seals and 813 members, including the previous
buffer baseline, without changing their frozen reviewers. Preserve qualification,
all live checks, original 300/60/10-second windows and first Causal refusals.
No material/fault/reconnect/server gate is promoted by this work.
