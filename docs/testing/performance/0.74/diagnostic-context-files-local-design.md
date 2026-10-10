# Account files opened under original Linux policy context

## Scope and original authority

Baseline is `81a3a5574121ea807218c23c5694c723dd6e71d8`. Add an opt-in Linux
reader that owns the observed-context guard and opens account files inside its
observation bracket. It borrows the original checked policy, current external
trust/envelope and caller assertions. It accepts neither preopened file guards
nor replacement context, numeric credentials, account snapshots or process FDs.
Fixed and temporary fixture origins remain distinct types; neither converts to
production preparation or authenticated start authority.

## Opening and later observation order

Construct the existing observed-context reader first. Then execute exactly
context, open account files, context, account-file observation, context. Context
steps revalidate original trust/policy and actual machine/boot/thread/user/mount
identity using the existing bounded reader. Open steps use the existing FileSet
descriptor/parser implementation with only private visibility widened inside the
policy module. Fixed input remains `/etc/passwd` and `/etc/group`, root-owned and
safe. Fixture input requires explicit nonroot ownership and rejects `/etc` and
descendants; ancestry is not production proof.

Later reads execute context, account files, context, with no reopening or adoption
of original FileSet objects. Existing file-name revalidation still reopens names
only to compare them with retained descriptors. Policy refusal precedes all IO;
an opening, context or account error stops the sequence, preserves the first
typed error and latches original policy plus composition guard. Restoration or
wrapper drop cannot reset refusal. A successful drop leaves the policy healthy.
No descriptors, context, policy or numeric mapping can be extracted. Both origin
wrappers inherit the original context guard's negative Send/Sync properties.

## Limits and tests

This bracket proves sequential opening-context consistency, not an atomic
snapshot or kernel-attested file-opener credential, initial-host authority,
all-thread enrollment, writer revocation or regular-file IO deadline. Real
issuer/epoch management and the join with namespace-checked kernel credentials
remain separate. No NSS, helper/IPC, live start, host mutation, workload, numerical
claim or qualification is enabled. Existing independent readers and all signed
schemas retain their behavior; rollback uses the old binary with this opt-in API
absent. Account-file budgets, mappings and context bounds stay unchanged.

Record baseline and missing API before implementation. Test exact opening/later
order, every failure position, prior refusal and first error, non-transferability,
constructor refusal after context mismatch or failed open, content/mapping drift,
replacement and restoration, policy revocation, distinct fixtures, seeded valid
foreign-context changes and independent concurrency. Positives combine actual
unprivileged Linux context with owned temporary account files under synthetic
issuer pins; they do not inspect a valid production worker account. Run scoped
Windows/Linux tests, affected check/strict lint, format, root contracts/evidence/
governance, docs/links/book and expected-red require-ship. Retain immutable raw
logs, preserve frozen qualification inputs, then commit code and evidence separately.
