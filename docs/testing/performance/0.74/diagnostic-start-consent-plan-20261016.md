# Explicit consent before collector start

Baseline is fdf0474f. A causal Heartbeat proves a response to a post-Reserved
challenge, not consent to launch. Keep its existing request/challenge schemas
and semantics unchanged. Preregister one separate, portable signed start-consent
byte contract before connecting the future consuming runtime owner.

Use distinct body/envelope schemas and Ed25519 domain. Canonical LF JSON binds
action start-cell-one, original controller attempt, retained Reserved record and
state hashes, causal challenge nonce, authenticated response hash, worker policy
digest/epoch, fixed collector unit digest, original controller/source/preset,
new request id, revision-two first-cell Starting state and a positive exclusive
Unix signing window of at most 60 seconds. Fields accept no path, command, FD,
verifier key, live peer or caller correctness clock. No old signature is
reinterpreted. Register the new opt-in matched-reader artifact in COMPAT.

Expose signing-message and bounded canonical byte/signature inspection only.
Inspection returns untrusted ordinary data, never an executable, lifecycle or
trust guard. Its independently supplied public key is only a signature-check
parameter, not an enrolled trust selector. Do not add an API claiming peer,
time, replay or execution admission from caller assertions. Body timestamps are
grammar here; actual freshness must later use the retained kernel owner.

First add a missing-module red test. Test real signatures, cross-domain/schema
confusion, every top-level/nested binding covered by the signature, malformed,
duplicate, unknown, noncanonical, future, partial and oversized input; exact
budgets, time/epoch integer bounds, first-cell grammar and concurrent independent
checks. Successful repeated inspection is explicitly not replay authorization.
Add negative API checks preventing byte data from starting/publishing/releasing.
Use deterministic seed 0x0740c0a2 and retain all failures without retry-to-green.

Only lib.rs registration and new module/tests change runtime sources. Add an
exact static contract freezing all original runtime bodies and keys. Layer a
narrow normalizer into the five existing historical static read closures to
remove exactly the new COMPAT row/main registration and those reader adapters
when comparing historical source. Verify the adapters restore byte-identically,
with negative canaries; never remove a prior assertion or accept arbitrary edits.

Run portable consent tests on Linux/Windows Rust 1.94/1.88, both compiler negative
API doctests, affected supervisor/xtask all-target/all-feature check and strict
Clippy on all four lanes, all static contracts, old Linux IPC/lease/original-fence/
host integration tests, formatting, docs/registry/local performance checks and
expected-red ship admission. No original chain is entered or changed in this
slice; ordinary 99-test original-input, manual kernel/causal/process-loss proofs
and full release verify are not new evidence here. Preserve the exact old files.

Next consume the causal owner, derive/freeze Starting from its original kernel
context and compare every consent binding under live trust/epoch/fence. Then
retain durable replay/uncertain intent before root output or authenticated
process start. Discharging bootstrap predicates, autonomous all-writer cessation,
completion/later cells and installed-host admission remain separate work. No
server, Actions, product workload, qualification, new timing claim or push.

Before a successful consent cohort: retain E0603 from an integration-test attempt
to call private canonical_json; reproduce its existing Value-then-Vec algorithm
inside the test, not by widening the library API. Also retain the six-pass/one-
failure development run: the test guessed more than forty scalar leaves, while
the selected 18-field body has exactly 36 nested leaves (controller four, state
sixteen). Replace that invented count with an exact eighteen-top-level-field
inventory and the audited 36-leaf count before running every mutation. No
performance/admission threshold, old test, workload or signing window changes.
