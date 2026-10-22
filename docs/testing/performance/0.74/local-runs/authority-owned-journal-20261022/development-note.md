# First reference lint refusal and authoring repairs

Reference source `227f1559`; owned candidate source `ddadb3a5`. Both first
reference images passed and their counts/traces are retained. The first Rust 1.94
reference gate then refused at strict clippy: manual_is_multiple_of suggested
replacing slot % 2 == 0 with slot.is_multiple_of(2). Rust 1.88 passed its complete
reference gate. The original red 1.94 receipt/log and reference source snapshot
remain red, not relabeled or overwritten. Candidate source makes only that
equivalent parity-expression repair in the frozen journal driver, plus its
explicit lifecycle module hook. All measured source identities stay distinct.
The first candidate lanes passed strict check/clippy and their seven tests;
the reference-only test was filtered rather than rerun. Measurement retries: zero.

Before measuring, development compilation found missing path attributes for
sibling modules, then unused imports and an unread error payload. The reference
constructor/reader controls were repaired before source freeze. Candidate
development compilation found an unused Scenario import and the required borrow
in owner() after changing its field from &Leaf to Leaf. These were repaired
before the first candidate test image ran. Console authoring failures are recorded
here, not represented as retained raw logs or failed performance observations.

Complete paired traces are compared for each fault/gate/drift target. Main
cohorts retain sixteen successful rounds and compare their terminal round trace;
the source-frozen deterministic workload is unchanged between rounds. Diagnostic
FNV trace checksums are not cryptographic evidence hashes; inventory/log SHA-256
seals supply evidence integrity. Synthetic slot routing/name, gates and logical
append/state/reopen are not filesystem/inventory/publication/identity proofs.
The external synthetic gate follows a full revalidation; direct latch cleanup
is source-guarded, not a new real external-event/owner-loss experiment.

No elapsed benchmark, CPU/op, physical-copy, RSS, Linux layout or admission claim.
The model contains no real descriptor, filesystem, manager, clock or transport.
Original material, all-writer, SIGKILL/lost-ACK, reconnect and server gates remain.

Final source review also distinguishes logical operator state examples from exact
engine error precedence. Model operator_refuse/advance test transition predicates
before append checks failed owner; actual FixtureAuthorityRegistry::change calls
revalidate first. Some already-failed states therefore return Invalid in the model
versus Refused in the engine. This is a source-derived model limitation, not an
executed runtime failure or a measured-image rerun. Keep the model unpromoted;
the exact engine integration gate must test/fix that precedence independently.
