# Native C allocation gate: P integration declined

Continuation of the [in-place MTU review](chitchat-inplace-result-20261029.md),
starting from `7bc4c4218acaf651d6a8d71edbde5ce12363d989`. The predeclared
[method and scope](../../../../tools/chitchat-native-cost-074/README.md) require
pointwise native C allocation dominance before spending CPU/RSS qualification.
**The gate refuses P in nine accepted cases. No product dependency is adopted.**
The previous Rust-only positive subgate is preserved, not promoted to overall
acceptance. No replacement optimization hypothesis is opened.

## Measured result

Release GNU Rust 1.94, original seed `0x0741028`, the same 239 ordered cases,
136 admissions, 103 rejections and exact original wire bytes. The only cohort is
four processes B/P/P/B, with two whole-operation observations per case per
process. B remains the known-wrong-bound cost counterfactual, never a product
repair. Both repetitions and both same-image processes match exactly.

| MTU | Value bytes | Extra native C allocations | Extra requested bytes |
| ---: | ---: | ---: | ---: |
| 40000 | 39938 | 6 | 718374 |
| 40000 | 39939 | 6 | 718378 |
| 40000 | 39940 | 6 | 718379 |
| 65507 | 65442 | 8 | 1134550 |
| 65507 | 65443 | 8 | 1134554 |
| 65507 | 65444 | 8 | 1134555 |
| 65507 | 65445 | 8 | 1134556 |
| 65507 | 65446 | 8 | 1134557 |
| 65507 | 65447 | 8 | 1134561 |

For example, MTU 40000 / value 39938 records 12 -> 18 C allocations and
1,436,748 -> 2,155,122 requested C bytes for the whole operation. All nine
regressions are accepted operations. Other cases show no native C difference.
Both logical Rust observer records and physically tagged Rust allocator records
match B exactly in all 239 cases. C peak requested bytes remain equal in every
case; both owners return to zero live requested bytes, without failed allocations,
external frees or counter errors. This is additional allocation/free work, not
a demonstrated leak. Equal C peak or balanced frees do not qualify allocator
caches, process RSS/PSS or retained memory.

P removes the writer/output clone but still predicts ambiguous admission by
compressing the pending chunks, then compresses again during actual append/finish.
The nine cases coincide with its nine exact-prediction invocations. Extra C
malloc/calloc/free calls are consistent with the three/four extra default Zstd
contexts per such operation. This is attribution of the existing candidate,
not a throughput or CPU claim and not evidence that public native API performance
has been measured. The corpus is a fixed-count codec diagnostic, not release W1.

## Observer qualification

Read-only inventory found GCC 13.3.0/glibc 2.39 in local Ubuntu WSL, without
heaptrack, Valgrind, perf or ltrace. The diagnostic uses GNU linker wrappers for
undefined malloc/calloc/realloc/free references, forwarding unchanged to libc.
No replacement allocator, with-rust-allocator feature or LD_PRELOAD library is
installed. Fixed per-thread counters and a bounded pointer table separate
Rust System calls by an explicit tag; the counter path allocates no heap memory.
Every overflow, ownership mismatch, untracked free or allocation failure refuses
an observation rather than being converted to zero.

The actual static Zstd archive imports malloc/calloc/free; the linked
`ZSTD_createCCtx` calls `__wrap_calloc`. Both actual Cargo feature sets are equal:
Zstd arrays/default/legacy/zdict_builder, safe arrays/legacy/std/zdict_builder,
sys legacy/std/zdict_builder. No vulnerable path is disabled to produce a pass.
The linker object, explicit GNU target, GCC/bfd selection and wrap flags are equal
in B/P. The scope excludes allocator internals inside dynamically linked libc,
aligned APIs not used by the selected Zstd archive, and process-wide heap state.

Seven standalone C control groups check preservation/zeroing, realloc failure
and zero-size behavior, errno, nested tags, bounded-table refusal and independent
threads. Both actual Rust images pass four controls each: the old allocator
control, C allocator control, opaque Rust tag control and original 239-case
semantic control. Four native observation tests then complete the cohort:
**12 selected native test executions**, plus the C control executable. Complete
catalogs have 118 B / 121 P cases; each variant executes five distinct names,
with 113/116 other names outside this gate. The previous full focused L3 gate
is separate, not reclassified as execution by these instrumented images.
All-target/all-feature release check and strict Clippy pass for both fixtures.
Ten inert fault/source-equivalence controls pass without executing native or archived code.

These are instrumented images. They cannot qualify clean CPU, RSS, startup or
production supported-target performance. Only GNU Rust 1.94 is observed here;
the previous unchanged P production-body 1.88 qualification remains separate.

## First refusals remain immutable

The first B/P builds attach global linker flags to host proc-macro shared
libraries and refuse local-exec TLS relocations. A separate explicit-target
context keeps the flags on target code. This follows the documented
[Cargo flag scope](https://doc.rust-lang.org/cargo/reference/config.html#buildrustflags)
and [GNU ld wrapping contract](https://www.sourceware.org/binutils/docs/ld/Options.html);
no TLS/allocator behavior is changed to force a shared-library link.

That context passes C controls but its first Rust tag control incorrectly wraps
the entire old allocator unit, including the old intentional panic. The outer
native region retains 232 Rust bytes and correctly refuses zero balance.
The old unit/assertions remain untouched. A separate scope context wraps only
the explicit allocation operations. It then exposes another important refusal:
logical Rust requests are 2/1/512 while native physical calls are all zero.
Read-only, image-hash-bound disassembly is retained; its tag calls and logical
counter updates support the inference that empty allocation work was optimized
away. Logical GlobalAlloc observations cannot stand in for physical malloc.

The final opaque-pointer/volatile control forces actual allocation use and
requires the original 2 allocations, 1 realloc, 512 requested bytes, peak 256 and
zero balance from both meters. This changes only the new control, preserving
all old tests and requirements. It passes before the first measurement. The
unchanged qualified C object is reused with fresh hash/source checks. Earlier
source capsules, controls, compiler streams and failures are retained separately.
Build caches may be shared across these sequential diagnostic source contexts;
only final qualified image hashes are current, and old image hashes/assembly
describe their retained historical versions. No old evidence file is overwritten.

Original envelopes stay 1200 seconds with 1100-second native sessions, physical/
native 4-GiB floor and five-second polling/retirement. All production live checks,
frequency/order and 300/60/10, tests 5/10/45, package 780/workspace 4800 remain exact.
No clock, existing service, supervisor, server, installation or product workload
is changed. Completed prior Windows L3 debug caches were removed, freeing
982,598,059 bytes. After sealing evidence, completed S/D release build caches
were also removed (558,568,966 bytes); source/evidence and B/P images were preserved.

## Decision and next boundary

P integration is **declined for native C cost**. No adopted product change needs
rollback; P's code, Rust success and this negative result remain review material.
CPU/RSS/allocator-retention cohorts are not run after the predeclared refusal,
and their gates remain unqualified. The existing live-writer capacity growth
64 -> 4099 bytes also remains documented independently.

Root lock remains `34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c`;
deny policy and qualification manifest are unchanged. Three product Zstd findings
remain open. A compatible accepted MTU repair, maintained source/public consumers,
actual Root policy/package/workspace gates and original execution/retirement/
reconnect/enrollment/host rehearsal remain necessary. This experiment supplies
no server or release admission and opens no replacement candidate.

All four source contexts, actual counter link objects, original ZIP inputs,
first compiler/control failures, image/catalog/feature/linkage evidence,
raw cohort streams and ownership receipts are retained in
[`local-runs/chitchat-native-cost-20261029/index.json`](local-runs/chitchat-native-cost-20261029/index.json):
1,714 logical members / 411 objects, index SHA-256
`3550fa5293b3f61f3b6d06662e3090b17658a42c1ca87c97b32f0c20e6d6c489`.
Compiled test images/build caches are excluded; source/object bytes and their
historical identities are retained. Final byte/doc verification is separate.
