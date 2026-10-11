# Original campaign fence local evidence

This packet checks short observation of the original fixed Reserved state under
the existing campaign flock. Positive cases are private nonroot models.
It neither enrolls the deployed worker-owned campaigns directory nor publishes
state, runs a worker, establishes durable campaign exclusion or qualifies 0.74.

Baseline is `7370c51a0ca23f4f9de1b6fad99dadecc201d16a`; the
[preregistered contract](../../diagnostic-original-fence-local-contract.toml)
and [design](../../diagnostic-original-fence-local-design.md) precede implementation.
The manifest binds the implementation commit/tree, LF source hashes and exact
raw captures. Run `C:/Python313/python.exe` with this directory's
`verify-packet.py` from the repository to verify those bindings.

The cases cover both existing legacy lock entry points and acquisition orders,
concurrent independent opens, healthy release, original/name substitution,
directory/state/lock metadata drift, missing or unsafe locks and all legacy
campaign/diagnostic and parent fixture contexts. Unsafe or bound failure refuses
the original storage permanently; initial Busy is retryable.

The explicit negative case succeeds at creating a legacy claim after the short
guard drops while fixed Reserved state remains. Thus this implementation cannot
authorize a publisher or start: campaign admission must first read the fixed
reservation under the same fence. Coordinated ownership enrollment remains
separate. No installer, server or frozen qualification input changes.

All red captures are retained: missing API before implementation, an expanded
test assertion initially placed in the wrong helper scope, and the new Linux
target initially missing from the gated-target registry. Only the test scope and
registry were corrected; no safety threshold or frozen contract was relaxed.
Final repeated tests and regression/lint receipts are distinguished by name.

Final unit repetitions each pass 10 cases. The selected Linux regression scope
passes 487 with one existing ignored; the public entry target passes one within
that scope. Eighteen compile-fail doctests pass. Windows portable tests pass 196,
and root registry/contract checks pass 154: 112 performance-contract cases,
13 release-evidence cases, six gated-target cases and 23 governance cases.
These overlapping scopes are not a unique release-wide test count.
