# Original controller verification under the common fence

The new opt-in preflight connects configured controller provenance and the
original common host fence without publishing state or admitting a nonce.
It is a read-only P2 prerequisite, not authenticated original-start. The
[preregistration](diagnostic-controller-fence-preflight-plan-20261015.md) and
[contract](diagnostic-controller-fence-preflight-contract.toml) fix this scope.

## Owned inputs and observation order

`OidcControllerInput::bind_common_empty_fence` exclusively borrows that original
guard and takes ownership of an already acquired OriginalEmptyReservationFence.
The OIDC guard retains the original token, request, reviewed build and controller,
worker policy/configuration, kernel packet/context and authority journal chain.
The fence retains its existing fixed storage and the same cooperative common
flock. No second lock, journal, root, caller timestamp or replacement input is
acquired by the composition.

Construction and revalidation run exactly fence/controller/fence/controller/fence.
Each controller check delegates to the existing complete verifier, including
original kernel freshness and policy/epoch checks. Each fence check delegates
to the unchanged original object, legacy-context and empty-leaf rules. These
are sequential brackets, not an atomic snapshot or a newly bounded total
operation deadline. Existing inner budgets remain unchanged.

## Refusal and lock lifetime

The first failing check stops the bracket and preserves its typed controller or
fence error. Both chains latch runtime refusal, including OIDC/configuration,
controller, worker, authority and original kernel packet. An already-refused
repeat returns Refused before any observation or logical checkpoint. This does
not write a durable authority-refusal record.

Constructor failure returns ControllerFenceFailure, which owns the refused
original fence. The common lock remains held until that failure is dropped;
there is no fence extractor or retry method. A later failure similarly keeps
the lock in the original preflight. Debug output reveals refusal/error status,
not token, request or descriptor contents. Healthy drop releases only the
original fence and leaves the original controller chain healthy.

The opaque result has no Clone, Deserialize, Send, file/token/request exporter,
replacement recheck, fixture promotion or start conversion. The production server
and existing early-attempt, publication and cell-consumption APIs do not consume
it. No executable mode, workflow, dependency, runtime wire or durable format changes.

## Local falsification and remaining writable join

Eight unit cases use existing private nonroot roots and synthetic issuer/signing
fixtures with actual local kernel IPC/context observations. They cover healthy
lock exclusion/drop, constructor cross-refusal and failure-owned lock retention,
all eight late legacy/leaf contexts, ten original input/kernel/policy faults,
four equal-object/name/mode replacement cases, all five bracket positions,
post-binding trust/authority drift and four independent concurrent chains.
Seed `0x0740c093` fixes the five-position mutation order. Refused repeats have
a checkpoint that would panic if observation resumed; restoration never repairs
the guard. Separate original lock witnesses check retention even when the lock's
public name is replaced. Nine compile-fail cases constrain the public API.

The [local packet](local-runs/controller-fence-preflight-20261015/README.md)
keeps the missing-method negative, focused repeats, regressions and source pins.
Synthetic positives do not enroll a real GitHub key, job or production filesystem.
There is no new persistent write and therefore no new SIGKILL publication boundary.

The separate [original controller attempt](diagnostic-controller-attempt-design.md)
now adds opt-in request retention by consuming this preflight. The constructor
and revalidation described above remain read-only; the new method is not used by
the old intent/publication APIs or any production consumer. Its acknowledgement
cannot authorize start, and its joined SIGKILL/lost-ack matrix is still required.

A writable lifecycle join must bind the same original request/nonce, policy/epoch
and common fence to an acknowledged durable attempt without releasing/reacquiring
the lock, opening another journal owner or reconstructing permission from bytes.
It needs explicit owned progression, uncertainty handling and SIGKILL/lost-ack
tests at every new write. Real runtime/issuer/job selection, authenticated worker
original-start, watchdog/all-writer cessation, completion/cells2-4 and server
admission remain separate. Empty preflight success supplies none of those rights.
