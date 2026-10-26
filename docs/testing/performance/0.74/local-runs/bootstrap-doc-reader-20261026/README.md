# Bootstrap document reader evidence

This packet retains a source-only compatibility step, not a full package or
server run. The baseline on `181c077abdfbae5a8c4dfe67cee6f1c95519d529` has
38 passes and sixteen failures across 54 cases: fifteen bootstrap failures and
one independent complete-audit failure enumerating 26 inputs.

The first candidate passes 69 selected cases on current/MSRV, then fails strict
Clippy. Its first correction fails compilation. Both outcomes and the original
baseline remain, with raw stdout/stderr and source/HEAD receipts. The final
candidate passes 53 original bootstrap cases, ten preceding proof cases and
six new controls on each compiler. There are no ignored or filtered cases.
Those cohorts do not complete the interrupted full package.

The immutable content-addressed index binds check logs, fixed predecessor caller
bodies, exact deltas, fixed current/predecessor document producers, final code
and coordinators. Earlier descriptor drafts are included without inventing a
run for the pre-check draft. Transient helper implementations have source hashes
in their receipts; this is not a claim to reconstruct every transient body.

The index binds 119 members in 99 distinct objects. Its SHA256 is
`7031d30ae5ab7f78609c5a96f96923b682cdeb7be600b6d1cd4c6e8b119a763f`.

Offline replay checks object integrity, exact first outcomes/case accounting,
unchanged source within each invocation, final current/MSRV/lint source bindings,
all eleven exact caller constructions and both inverse document projections.
It executes no retained source, Git, Cargo, fixture or workload:

```text
python docs/testing/performance/0.74/local-runs/bootstrap-doc-reader-20261026/verify.py
python docs/testing/performance/0.74/local-runs/bootstrap-doc-reader-20261026/test_verify.py -v
```

Checks retain the 1,200-second bound and four-GiB disk floor. No runtime,
dependency, live-check frequency/order, admission deadline, numerical threshold,
clock, service, server or qualification changes. No private keys or binaries are
included. See the [result report](../../bootstrap-doc-reader-result-20261026.md)
for the narrow scope and unresolved 26-input audit frontier.
