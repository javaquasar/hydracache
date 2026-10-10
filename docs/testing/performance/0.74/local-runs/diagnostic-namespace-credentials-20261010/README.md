# Local namespace checked credential evidence

This packet retains local Linux opening/read sequencing and refusal checks,
not production host enrollment or release qualification. The original numeric
credential API and account binder remain separate and unchanged. The new reader
owns both guards and opens status only inside its own namespace bracket; no
preopened document can be enrolled.

The baseline passes 230 Linux tests. The missing-API capture refuses before
implementation; its first shell wrapper fails to record the exit variable.
The second capture records Cargo's actual native exit 101; the surrounding
wrapper incorrectly expected 1 and refused. Both captures are retained as
negative command evidence, not behavioral test failures. Ten initial new tests
pass; review adds pre-refused component and invalid private opener-state cases.
The final Linux suite passes 242 with one pre-existing ignored, and all twelve
new cases pass three repetitions. Windows passes 28 portable tests and executes
no Linux namespace cases. Root contracts/evidence/governance pass 126 checks.

Real positives use owned non-product no-new-privileges cat helpers and the
private original-process test pin, not a production diagnostic cgroup. Exit,
status FD/named-document and namespace FD substitution/restoration, wrong
assertions, unhardened credentials and concurrent independent readers exercise
kernel observations. Constructor/failure-stage and seeded error variations are
modeled, not real foreign namespace transitions. Seed `0x74f2026` varies 256
first-error/stage cases per repetition. No namespace creation or join occurs.

The bracket is sequential, not inspection/attestation of kernel file credentials,
atomic or continuous namespace proof, all-thread policy, trusted NSS/initial
host namespace or authenticated original start. No host unit/account/install,
product/observer workload, numerical performance measurement or qualification
ran. Require-ship remains expected red and the frozen qualification digest is
unchanged. Full workspace milestone verification was not rerun.

Run `python verify-packet.py` in this directory for read-only raw size/hash,
test rows, seed, scope, admission and frozen-digest verification. The manifest
records exact source/tree and twenty byte-preserved captures. The working-tree
observations are local safety evidence, not clean-source qualification receipts.
