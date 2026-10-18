# Boot ID buffer candidate

## Hypothesis and frozen baseline

Baseline `3b8e73feec8412e4090bf05575f06a05901e1010` reads the canonical
37-byte boot ID through `bounded`, requesting a 65,537-byte heap buffer. The
kernel walk packet attributes 32 boot reads to each whole worker validation.
This is a buffer hypothesis, not permission to remove any validation. No product
throughput or whole-worker allocation gain follows from the source inspection.

The sole runtime candidate replaces that temporary buffer with a 38-byte stack
buffer: the canonical document plus one overlength sentinel. It retains one
positional read at offset zero, without EOF, short-read or EINTR retries. File
opening, procfs checks, descriptor ownership, UUID syntax, fresh observations,
refusal latching, effect brackets and the original 300/60/10-second origins and
deadlines remain unchanged. Status and pidfd documents still use `bounded`.

## Semantic boundary

Test the actual boot decoding path against the frozen algorithm with empty,
short, canonical, uppercase, non-ASCII, misplaced hyphen, missing newline and
overlength documents, every single-byte mutation, and deterministic generated
documents. Inject read errors and short reads and assert exactly one call. In
particular, an EINTR is an error, not a retry. A canonical prefix with a trailing
byte must refuse. A read returning only the canonical prefix cannot prove EOF
in either algorithm; this candidate must not add such a claim.

Public results must remain the same (`KernelError::Invalid` or the original
I/O error). The old test-only site for empty input remains
`proc-document-budget`. An oversized input that previously reached the
65,536-byte budget may now refuse as `boot-document`: the small read does not
observe its total length. Diagnostic-site identity for that malformed class is
not an authority predicate and is explicitly not an equivalence claim.

## Measurement lanes and acceptance

Use a separate integration-test image for allocation counting, with a
thread-local counting allocator confined to that image. Compare the exact new
reader with the old one-read buffer algorithm on identical bytes, offset and
read count. Drop returned storage inside the measured closure. Repeat 32 reads
in each of five independently entered scopes; require one allocation and
65,537 requested bytes per baseline read and zero allocations for the new
temporary reader. These are requested allocation bytes, not physical zeroing,
RSS, copied bytes, or whole-worker allocations. The returned boot String and
path allocations are outside this narrow measurement and remain present.

Keep timing outside that allocator image. A plain same-host original-worker
A/B and full preparation healthy/fault checks are required before accepting the
candidate into the approved preparation path. Preserve old packets; bind each
new source/image separately. Instrumented kernel Document spans must retain
their count and cover temporary storage initialization and the single read,
not the UUID parser or returned String allocation. Stop at the first unexpected
host failure, retain it and withdraw only this runtime candidate if necessary;
do not extend deadlines or retry into success.

Local acceptance for an experimental branch commit requires differential and
fault tests, allocation controls, exact source-contract restoration of the old
boot function (all other old runtime statements identical), formatting,
affected-package check and strict Clippy, registry, docs and local
non-promotable performance contracts. It is not runtime promotion or a release
gate. No install, service action, product workload, trust enrollment or
qualification is authorized by this experiment.

## Status

Preregistered before implementing or measuring the candidate. The approved
preparation source remains `2ed04dc866b39fefea8d81be2328f7f1382e3d46` until the
plain matched and full preparation gates pass.
