# Original authority and signed expected intent local evidence

This package composes a signed intent with the exclusively borrowed original
authority chain. It does not publish fixed intent, prove a trusted host fence or
persisted state, authenticate a live peer/clock, enroll keys or authorize a worker.
The [preregistered design](../../diagnostic-authority-intent-local-design.md)
and [contract](../../diagnostic-authority-intent-local-contract.toml) define this scope.

The absent public API failed before implementation. Baseline authority and
standalone projection targets passed. Initial eleven unit cases passed; two
additional refusal cases and four compile-fail API examples then completed the
intended scope. The first format invocation preceded creation of the tests module
and failed; that terminal-only failure is not represented as a retained capture.
The initial root gate named a nonexistent `test_governance` target and was
rejected before running tests. Its raw failure is retained; the corrected gate
uses the existing `release_governance` target without altering a test.

Owned temporary Linux journals and helper processes test the original chain.
The positive fixed-dispatch case uses private test-only model seams, not enrolled
root storage. Four publication fault checkpoints model partial and
late-unacknowledged writes, not real process/power-loss crashes. The two
publication uncertainty cases invoke a private rejection seam after binding.
Independent concurrent chains do not prove continuous whole-host confinement.

The manifest retains exact raw capture hashes, recorded source/tree identities,
commands and scoped test totals. Three focused repeats have separate literal
filenames. Historical storage evidence remains bound to its own recorded source;
its current-source equality check is intentionally not applicable after this
composition changes shared tests and intent code.

Implementation is `72aa64a17e453643bdd37c15353161013bc393b3`, following
preregistration `e0217e55dba0d10435246dfef5a60be28e130a5d` and baseline
`f96912725f8d7fa4b3501f548d5bd0d518d5a483`. Thirteen new unit cases pass all
three separately retained repeats. The selected Linux regression scope passes
465 with one existing ignored; Windows passes 196. Root checks pass 110
performance contracts, 13 evidence, six gated-target and 23 governance cases.
Twelve compile-fail doctests, strict clippy, MSRV 1.88, rustdoc with denied
warnings, formatting, documentation and the local non-promotable contract pass.
These overlapping scopes and repeats are not unique release-wide totals.

Twenty-nine raw captures include both genuine negative gates and the rejected
root invocation. The packet verifier checks their exact bytes, Git source and
tree identities, new W11 tests, historical storage sources and all eight frozen
qualification inputs. Its initial mdbook phrase expectation did not match the
successful tool's actual `HTML book written to` output; the expected marker was
corrected without altering the captured log or command exit status.
The final root/documentation/book checks are retained separately from the initial
verification. They do not add distinct test cases to the scoped totals above.

Run the read-only packet verification from the repository root:

```text
python docs/testing/performance/0.74/local-runs/diagnostic-authority-intent-20261011/verify-packet.py
```

Frozen qualification inputs and the inherited plan-digest mismatch remain
unchanged. Local contracts can pass without qualification or ship admission.
Next are the original host fence and trusted persisted Reserved state, fixed
intent publication, then production output/start/watchdog/owned cleanup and real
issuer/host enrollment. No product performance claim follows from these tests.
