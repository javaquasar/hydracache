# Original user namespace local evidence

This packet checks opt-in same-user-namespace consistency between the retained
original process and the current reading thread. It is not initial host/NSS
enrollment, credential-opener proof, a production start or numerical performance
evidence. See the [design](../../diagnostic-user-namespace-local-design.md).

The [manifest](manifest.json) binds 20 raw captures by size and SHA-256. Run
`python verify-packet.py` to check byte retention, exact test counts, API and
implementation negatives, expected-red ship admission and the frozen
qualification digest. The verifier is offline/read-only and launches no worker.

Baseline Linux coverage passes 218 tests with one pre-existing ignored. After the
missing API and missing-trait-import checks, eleven new tests pass. Review adds
the kernel-object identity comparison; final coverage passes 230 and all twelve
namespace cases pass three more repetitions. Windows passes 28 portable tests
and executes no Linux namespace cases; root contracts/evidence/governance pass
125 checks, followed by one post-registration check.

Owned cat processes demonstrate repeated same-namespace observation and refusal
after original process exit even while namespace descriptors survive. Private
descriptor/directory substitution tests remain refused after restoration. Real
mount namespace and ordinary-file descriptors fail the nsfs/user-type policy.
Concurrency uses independently retained guards. Foreign-user-namespace drift
and seeded device/inode mutations are synthetic; no namespace was created or
joined. The test-only original-process pin does not prove production cgroup
identity. Credential opener, trusted provider/host policy and signed lifecycle
remain separate prerequisites. No account/unit mutation, host install, product
workload or qualification ran.
