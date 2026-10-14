# Dedicated diagnostic controller job preparation

The [owned nonroot child root probe](diagnostic-signer-root-design.md) now checks
kernel root/cwd, a child-only user namespace/map, zero capability sets and fixed
absolute file identities in a temporary tree. It performs no mounts or signer
exec; current root selection is not loaded-runtime or protected-job custody.
The [local packet](local-runs/diagnostic-signer-root-20261015/README.md) cannot
activate this job. Join the root to original exec and actual loader/library
objects, and address later namespace/root escape before handoff integration.

The new [bounded runtime inventory](diagnostic-signer-runtime-design.md) adds
declared ELF dependency and original readonly file checks for the fixed GNU
helper, loader, libc and libgcc copies. It neither forces launch to use those
copies nor authenticates their production custody. The exact-source
[local evidence](local-runs/diagnostic-signer-runtime-20261015/README.md) is not
a permission, token, protected-job or worker-start grant. Enforced selected
loader/root and runtime/launch composition remain the next local boundary.

The approved local bootstrap retains two standalone test-build mechanics:
[owned first exec](diagnostic-signer-first-exec-design.md) and
[exact seccomp installation](diagnostic-signer-seccomp-design.md). The latter
observes original bytes/TSYNC/zero exit and denial on a single-thread raw-fork
helper; it has no fresh signer exec, runtime admission or handoff consumer.
The newer [joined fixture](diagnostic-signer-joined-design.md) observes first
exec and exact installation on one owned fresh GNU helper and tests child
SIGKILL at two boundaries. It still has no admitted runtime, handoff or actual
tracer-loss proof. These fixtures cannot activate this job. Runtime custody must
precede dummy handoff; actual job-wide permission and private-key custody still
require the independent enrollment below.

The fixed public source was received once on 2026-10-14; see the
[public acquisition packet](local-runs/diagnostic-oidc-public-source-20261014/README.md).
This prepares the next job boundary under the
[approved local OIDC decision](diagnostic-controller-provenance-decision.md).
No active workflow, protected environment, signing secret or token request is
created. The campaign and separate builder workflows remain unchanged.

## Existing fixed profile and missing selections

The receiver profile already requires repository ID 1217101761, the dedicated
`diagnostic-controller-074.yml` workflow on `feat/0.74-resp-native-throughput`,
environment `performance-diagnostic-controller-074`, workflow_dispatch and a
GitHub-hosted runner. Its workflow SHA and exact subject must be selected
independently; neither is filled from the observer commit or default subject
syntax. Current GitHub subject settings can differ from older name-only forms;
select the actual repository policy rather than guessing. GitHub documents
[OIDC claims and subject configuration](https://docs.github.com/en/actions/reference/security/oidc).

The historical builder environment/secret has a separate fixed-build purpose.
Its retained October 9 protection receipt is not protection for this new
controller environment. Reusing that key or environment would not establish the
new job's custody. The controller public pin remains the existing reviewed
`c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda`;
private key custody for this job requires separate enrollment, not a key read here.

## Public key selection decision before job activation

The authenticated transport report contains four RSA/RS256 public keys. The
current v1 trust reader accepts exactly one explicitly selected key. Discovery
and JWKS do not identify which one will sign a future job token. Neither roster
order nor certificate metadata selects an active kid or a snapshot lifetime.

The operator subsequently approved local implementation of the
[bounded v2 roster](diagnostic-oidc-roster-design.md). V1 remains readable with
one explicit key; v2 accepts one through eight independently configured keys
within 8192 canonical LF bytes. The token kid identifies an already configured
member, never a new key. Duplicate kids/material, unknown kid, stale snapshots
and source replacement refuse without network refresh or fallback.

Actual production membership, workflow/subject and snapshot lifetime still need
independent selection and custody review before activation. The public packet
is not installed or automatically promoted into either runtime configuration.

## Job sequencing and custody

1. Review one tooling/workflow revision and confirm repository/ref/environment
   protection before exposing token or signing authority. Restrict token permission
   to the signing job, not an untrusted build or reusable workflow. GitHub requires
   `id-token: write` for token retrieval; see its
   [OIDC permission reference](https://docs.github.com/en/actions/reference/security/oidc#workflow-permissions-for-the-requesting-the-oidc-token).
2. Build and verify the signer in a distinct job without id-token permission or
   signing secrets, with a sanitized build environment. Token permission is
   job-wide, not a safe late-step grant; clearing variables in one build command
   cannot remove the job's authority. The signing job therefore needs an
   independently authenticated prebuilt signer/runtime and reviewed bootstrap.
   Keep source/runtime/lock receipts; no compilation, dependency install or
   unreviewed checkout after authority exposure. Review the original executable,
   not just an artifact name or self-reported hash. The frozen observer builder's
   fixed-build receipt does not attest the controller signer; do not expand that
   key purpose implicitly. This bootstrap/artifact custody join remains code work.
   The [separate signer receipt model](diagnostic-signer-build-design.md) checks
   a new domain/role and supplied bytes locally, but caller-asserted selection is
   not independent custody. It has no job/producer authority consumer and does
   not replace this pre-exposure bootstrap requirement.
   The [original signer-file reader](diagnostic-signer-bundle-design.md) retains
   real fixture descriptors with sticky name/metadata/hash checks. Its asserted
   root/selection and opaque context still cannot attest the original process
   mapping, loader/runtime or protected job. No secret or token grant consumes it.
   The later [kernel executable observation](diagnostic-signer-executable-design.md)
   compares the original procfs object with that retained binary. Its same-inode
   re-exec counterexample means it cannot replace original invocation evidence,
   loader/runtime integrity or independently authenticated pre-exposure custody.
   The [bootstrap decision](diagnostic-signer-bootstrap-decision.md) proposes a
   separately trusted launcher/runtime with kernel first-exec observation and
   post-launch exec restriction before handoff. The operator approved local
   mechanics only. The first [owned exec-stop fixture](diagnostic-signer-first-exec-design.md)
   now exists in test builds; it has no handoff and cannot certify the
   token-bearing job's runtime/custody or authenticate worker start.
3. Load independently reviewed builder policy/receipt and admitted lease/boot
   context. Define one operation, revision, request ID and nonce. Existing local
   CLI validation cannot authenticate supplied host context or consume a nonce.
   Generate the fresh request only after slow preparation, preserving its
   exclusive maximum 60-second window.
4. Freeze one canonical body and derive audience from it. Retrieve the token for
   that audience through a separately reviewed bounded HTTPS producer. Never
   reuse the public-acquisition helper for bearer-token transport: it refuses
   token environment names and is deliberately a different boundary.
5. Validate run/attempt/actor/repository and exact workflow/subject/environment
   against the original body and independent selected issuer snapshot, then sign
   that same retained body. Current audience/sign CLI calls reread files. The
   local [retained signing API](diagnostic-retained-producer-design.md) now
   consumes one prepared object and checks the earlier audience before key IO;
   the future coordinator must keep that object through actual retrieval and
   verification. Matching audience text alone is not token proof. Never log or
   archive raw tokens/keys.
6. Hand off only through the separately enrolled transport and original receiver
   chain. No SSH shell, arbitrary command, replacement body or direct fixture-to-
   start conversion. Timeout, uncertain delivery or lost acknowledgement retains
   the attempt; it cannot renew expiry or create automatic execution retry.

Actor ID names the workflow initiator, not an independent rerun approver.
GitHub distinguishes the original actor from the triggering actor on reruns;
see the [variables reference](https://docs.github.com/en/actions/reference/workflows-and-actions/variables).
Protection and rerun policy require explicit review. In the operator-plus-agent
model, technical agent review and one human environment approval must not be
presented as two independent human reviews.

## Code and tests still required

The producer validates/signs retained job bytes, including body/audience
consistency without public-file reopening. It does not yet implement token
retrieval/verification coordination, signer/bootstrap custody, an authenticated
receiver transport or a protected-job receipt. Preregister each before
implementation. Cover denied
permissions/protection, wrong workflow/source/subject, key rotation/unknown kid,
audience/body substitution, rerun attribution, token/packet expiry, disconnect,
lost ack and no raw credential retention. Positive local models cannot replace
the first separately approved actual protected-job receipt.

Original worker policy/body pin/current epoch, physical host/root custody,
durable nonce/common fence and authenticated start remain required at the host.
This job design opens neither completion/later cells nor server/pilot/qualification.
