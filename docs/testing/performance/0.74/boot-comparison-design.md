# Allocation free fresh boot equality

Inherits `docs/RULES.md`. The [plan](boot-comparison-plan-20261019.md) tests one
ownership change within the existing preparation/enrollment validation path.
It does not change the validation graph, observation frequency or time bounds.

## The measured target

The closing attribution counted 6,688 Boot observations per completed round.
Boot work includes actual procfs traversal and decoding; its inclusive wall span
is not an exclusive allocation cost. The next hypothesis therefore removes a
known temporary allocation, without claiming that it dominates the elapsed
preparation budget. The frozen reference measures complete origin revalidation
and sampling through fresh procfs and namespace objects.

The baseline makes two 36-byte Rust allocation requests per revalidation and
five per sample. Sampling validates twice and also owns the boot ID in its
returned time point. The optimization keeps that final ownership; it removes
only the temporary strings used to compare fresh boot documents against the
already retained boot ID.

## Every observation is still fresh

The same traversal opens sys, kernel, random and boot_id with the original
flags and filesystem checks. It performs one bounded positional read, closes
the document before decoding and never retries a short read or IO error. The
decoder still rejects zero/incorrect lengths, missing newline, wrong hyphens,
uppercase or non-hex bytes, and checks UTF-8. The decoder stays in its original
module so existing mutation guards continue to inspect its actual predicates.

A private generic function applies either owned decoding or borrowed equality
to that just-read stack buffer. Only the equality result leaves the callback;
neither a borrowed UUID nor a fresh descriptor is cached or retained. Boot and
Document trace spans retain their original calls and decoding scope. The
retained origin and sampled time-point fields and public API are unchanged.

An exact source adapter reconstructs all 420 prior source bodies, not just the
changed function. It rejects missing/duplicated substitutions and independent
drift in flags, metadata, clock selection and revalidation calls. The actual new
helper and frozen reference decoder are separately pinned. Differential tests
cover 9,728 byte mutations, lengths including oversized counts, 1,024 seeded
documents, read errors, real procfs, refusal sites, sticky drift and original
live observation counts.

## Allocation evidence and the fault gate are separate

The existing opt-in counter observes Rust allocation requests, not libc malloc,
RSS, exclusive CPU or throughput. Three pairs of 1,024 calls per operation are
fixed before each measured cohort. Ordinary unprofiled libtest images, after
all builds and source checks finish, supply the separate MSRV creator fault gate.
Neither a lower allocation count nor a completed local fault cohort opens
production enrollment, authenticated original delivery, lifecycle completion,
fence release, server installation or product workload.
