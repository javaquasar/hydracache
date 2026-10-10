# Local fixed account and namespace checked reader binding evidence

This packet retains asserted mapping consistency and error propagation among
original account, namespace and credential guards. It is not trusted host/NSS
enrollment, helper namespace attestation, production composition or qualification.
The numeric-only API, older binder, namespace-checked status opening/read order
and frozen qualification inputs remain unchanged.

The Linux baseline passes 242 tests. A missing-API library check refuses with
Cargo native exit 101 before implementation. All twelve new tests pass first
green and three further repetitions. Final Linux coverage passes 254 with one
pre-existing ignored. Windows passes 28 portable tests and executes none of the
Linux binding cases. Root contracts/evidence/governance pass 127 checks before
packet registration; the registry is checked again after retention.

Real owned NNP cat helpers exercise original namespace/credential readers,
success/drop, original exit, public early-refusal and pending-helper reservation,
with synthetic fixed-account projections. Account drift, observation ordering
and first-error injection are modeled. Seed `0x7502026` varies 256 canonically
valid UID/GID/membership mutations per repetition. Positive account projections
do not authenticate a real NSS account or provider. The independent real NSS
operator still refuses with exit 9, empty stdout and confirmed helper cleanup.
No test creates or joins a namespace or mutates a host account/unit.

The binding preserves typed first errors and refuses all original inputs through
constructor failure and borrow-wrapper drop. Restoration cannot repair refusal;
successful drop preserves healthy inputs. No arbitrary callback is public and
no numeric-only reader can be converted into a namespace-checked one.

Scoped format/check/clippy and documentation checks pass. Require-ship stays
expected red; the qualification digest is exact. No full workspace milestone
verification, host install, product/observer workload, numerical performance
measurement or expensive qualification ran. Nineteen byte-preserved captures
are local working-tree safety evidence, not clean-source qualification receipts.

Run `python verify-packet.py` here to check raw sizes/hashes, test rows/cases,
seed, scope, refusal and the frozen digest without starting any process workload.
