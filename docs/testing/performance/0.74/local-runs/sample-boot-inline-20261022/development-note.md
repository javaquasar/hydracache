# Source guard repair history

The first candidate host gate returned 101 with two passing new canaries and
one failed new canary. The restoration assertion inserted a second trailing
newline into the unchanged reference probe. The raw failure and exact failing
source are retained under candidate. Only that assertion was corrected; rustfmt
collapsed it to one line. The source-only closing cohort under
candidate-canary-repair then passed all forty-six selected canaries and xtask
check/strict clippy. Both compiler measurement lanes remained untouched.

The first offline reader development invocation ran zero tests because its
assertion-repair comparison accounted for the newline substitution but not
rustfmt's one-line formatting. It raised ValueError with reason "Only exact
newline guard repair permitted". The reader was corrected to require the one
exact old multiline assertion and its exact new one-line replacement, then
passed its eight offline mutation controls. This note records that development
failure; it is not a raw subprocess log or a new benchmark observation.

No live failure, original Causal(Time) refusal or admission deadline was repaired,
relabelled or retried. The two first measurement selections and every interval
remain in the packet, and the inline representation is rejected for adoption.
