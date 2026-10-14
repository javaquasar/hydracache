# Original controller request and reviewed builder receipt binding

Baseline is `5c84fb7f15077ec5841be4e6d8e4a314a710e575`. This bounded P2 step
binds the original signed controller request to a receipt from the separately
reviewed builder authority. It authenticates a build statement, not GitHub run
provenance, installed executable contents or host execution admission.

## Retained inputs and failure behavior

Add `SignedControllerInput::bind_reviewed_build(policy_bytes, receipt_bytes)`.
The original controller guard brackets verification with its existing kernel,
worker policy, authority journal, controller config and request checks. Load only
the independently pinned builder policy through `diagnostic_reviewed_trust`;
no caller key, digest, identity or trust override is accepted. Verify the bounded
canonical signed receipt against the identity already in the original request.

Return a private-field read guard borrowing the original controller guard and
owning the checked builder policy, verified statement and copies of the original
bounded policy/receipt bytes. Revalidation takes no replacement arguments and
rechecks those originals, including statement/identity equality. Every error
latches the controller and original worker/config/journal/packet chain. Healthy
drop preserves owners. There is no extractor, clone, thread transfer, serializer,
start, nonce consumption, fence acquisition or completion capability.

## Provenance limits and next join

The existing build statement has no controller run ID, run attempt, actor,
workflow or environment fields. A valid builder signature therefore cannot
authenticate those controller assertions. Do not add fields to its wire schema
or manufacture provenance by comparing two assertions. Independently reviewed
controller provenance must precede durable replay and common-fence admission.
Actual worker policy/epoch, root/source-opener enrollment and installed binary
verification remain separate prerequisites. Execution and server entry stay
closed; frozen inputs, selected keys and existing durable formats stay unchanged.

## Local verification

Capture the existing controller and reviewed-builder baseline, then the missing
binding API compile refusal before implementation. Positive tests combine the
existing public historical builder receipt with synthetic controller/worker
signers, private nonroot config/journal roots and actual local Linux messages.
Never read private keys, sign real production policy or install configuration.

Test identity/digest/signature/policy mismatch, empty and over-budget inputs,
retained-byte and original-owner drift, sticky whole-chain refusal, healthy drop
and independent bindings. Use seed `0x07542074` for signed receipt mutations.
Check compile-time extraction/start/replacement/borrow/thread constraints. Run
three focused repeats, affected regression and doctests, current/MSRV checks,
strict current lint, scoped formatting, root contract/evidence and documentation
gates. Reuse the registered Linux worker/kernel integration target. No new
process-loss proof or performance claim belongs to this read-only step.
