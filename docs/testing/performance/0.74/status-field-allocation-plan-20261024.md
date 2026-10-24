# Separate status-parser allocation contract

The accepted fresh-field index has a retained pure-parser CPU result, not an
allocation/RSS result. Do not repeat its CPU cohort or infer memory from elapsed
time. Add a separate dependency-free allocation control over the already retained
exact baseline/candidate parser sources. No runtime/product change belongs here.

Freeze this method before building either control. Verify the original build
receipts and every extracted source byte before copying; never regenerate an
old baseline from the changed runtime. Reuse the exact dataset generator,
credentials, shapes 512/2048/8192, eight shuffled documents and seed 0x07405ca1.
The new harness must have the same bytes in baseline and candidate. It must not
read procfs, clocks, manager state or external inputs, create a worker or start
an original owner. GNU/MSRV builds execute once per lane, with no retry-to-pass.

Instrument the Rust global System allocator only in the single-threaded parser
interval. Count successful allocation/reallocation calls, gross requested bytes,
deallocations, peak requested live bytes, bytes held by the returned result and
bytes still live after dropping it. Dataset construction, formatting and output
are outside the interval. Validate allocator accounting with explicit owned
alloc/zeroed/realloc/free operations before parser controls. Retain both the
held-result and after-drop snapshots, not just a zero final number.

For every document, cover valid input, malformed early scalar, duplicate/missing
field, malformed group token, wrong credentials and group widths 0/1/4/32/33.
Compare semantic outcomes and complete per-case allocation transcripts exactly
between baseline and candidate within each compiler. Acceptance requires no
increased requested allocation/retention at any case and zero outstanding bytes
after drop. An allocator-control failure or transcript difference stops the lane;
preserve its first result rather than revising the oracle to match it.

These are optimized extracted-parser requested-heap observations, not allocator
arena retention, RSS, stack high-water use, whole-runtime allocation counts or
product memory evidence. No native API, original preparation margin, server or
release admission follows. Existing live predicates/count/order/windows and
first failures remain unchanged; no clocks, services or server actions occur.
