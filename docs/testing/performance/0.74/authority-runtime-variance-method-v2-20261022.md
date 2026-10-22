# Future local authority timing protocol with corrected cohort counts

Status: controlling prepared protocol, not implemented or executed. Read this
with the [archived first draft](authority-runtime-variance-method-20261022.md).
All source pins, workload/check order, per-process context, estimator, admission
boundaries, fixed budgets and no-retry stopping rules in that draft remain.
Only its two arithmetic sample-count labels are corrected here; the proposed
block schedule itself does not change.

## Exact sample membership

Eight blocks per compiler, four processes per block, two compilers:
8 × 4 × 2 = **64 measured processes**, not 128. Each compiler has 32 processes:
16 baseline and 16 installed, not 32 observations per role/compiler. Each block
contains two observations per role; the endpoint estimator still has exactly
eight block ratios per compiler. The clock/semantic controls and builds precede
this cohort and are not counted as measured processes.

For each compiler, block indices 0 through 7 are fixed as
BIIB, IBBI, IBBI, BIIB, IBBI, BIIB, BIIB, IBBI. Within each index execute both
compiler blocks serially: 1.94 then 1.88 on even indices, reverse on odd ones.
Across each compiler's schedule both roles occupy every position four times.
Each process retains the original 12 ordered cells, seed 74 and 32 inspections
per warm cell. Thus a complete future cohort has 768 cell rows and 24,576 timed
warm inspections, with sticky/internal checks additional and unchanged.

The first draft and its sealed snapshot retain the erroneous count labels as
preparation history. They are not execution instructions for a larger cohort.
No measurement was run under either draft, and no old CPU observation changes.

## Freeze and execution boundary

Before any future execution, require an offline schedule guard for exactly
64 processes, 16 observations per role/compiler, eight block ratios/compiler,
four occurrences of each block orientation/compiler and balanced role positions.
Freeze its schedule digest with the separate runner/probe. A count mismatch
must refuse before measurement, not trigger adaptive sample selection.

The unchanged budgets are 300 seconds per measured process and 60 minutes for
the complete measurement cohort after builds. The original production 300/60/10
windows, all live checks, first refusals and qualification remain untouched.
No implementation/execution, clock/service change or server action is authorized
by preparation of this corrected protocol.
