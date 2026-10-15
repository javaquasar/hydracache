# Original controller checks under the common fence

Baseline: `5ae10682`. This is a local P2 prerequisite under the approved
server-readiness sequence, not durable replay admission or authenticated start.
Retain the existing OIDC input baseline before adding code or tests.

Add a separate opt-in opaque preflight owner. It exclusively borrows the original
OidcControllerInput (and thus its original builder/controller/worker/kernel/config/
authority chain) and owns an already acquired OriginalEmptyReservationFence.
It must not acquire another journal, lock or namespace, reopen/adopt an attempt,
select trust values, or expose a request, token, policy, FD or state conversion.
Keep the old APIs and existing verification/rejection conditions unchanged.

Constructor and revalidation check fence, controller, fence, controller, fence.
Controller checks use the same retained request/token and original kernel clock;
no caller peer/time or replacement inputs. A first error stops observation,
preserves its category and permanently refuses both chains, including their
original config, authority and kernel packet. A refused repeat performs no IO.
Constructor failure returns an opaque owner of the refused original fence, so
the common lock stays held until that failure is dropped. Healthy destruction
releases only the original fence and leaves the controller chain healthy.

The preflight publishes nothing and adds no SIGKILL write boundary. Empty-fence
eligibility is not nonce consumption, an acknowledged attempt, trusted production
enrollment, a whole-operation deadline or an execution capability. Existing
durable attempt/publication/start paths must not accept this result. Integration
with those paths requires a later explicit owned progression and crash matrix.

Test first with the missing method. Reuse the existing synthetic OIDC RSA and
controller/worker fixtures plus actual kernel packet/context observations; use
only private nonroot storage roots. Cover healthy original lock exclusion/drop,
already-refused inputs, late legacy/fixed-leaf contexts, original lock/object
replacement, OIDC/source/worker/kernel/authority faults, restoration refusal,
typed first errors and every logical bracket position. Seed 0x0740c093 must replay
mutation order; independently created concurrent chains must remain isolated.
Compile-fail checks prohibit Clone, Deserialize, Send, input refresh, extraction,
fixture promotion and start. No new public test fixture constructor.

Run repeated focused Linux tests, original input/fence regressions, doctests,
all-target/all-feature checks and strict lint on current/MSRV Linux and Windows,
format, static performance contracts, registry and doc/local-admission checks.
Retain baseline, red, source identities, raw outcomes and all negative results;
commit implementation separately from evidence/article. Preserve Cargo.lock,
qualification inputs, old packets, product code and frozen Release0.73. No actual
key/token, host/GitHub operation, product workload, push or server admission.
