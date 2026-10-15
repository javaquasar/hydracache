# HydraCache 0.74 diagnostic enrollment review

The October 15 read-only inventory identifies the existing server, accounts and
GitHub policy needed to prepare real diagnostic trust. It also exposes a receiver
namespace mismatch: SSH host facts cannot be copied into a supervisor policy.
The [evidence packet](local-runs/diagnostic-enrollment-inventory-20261015/README.md)
and [machine-readable proposal](local-runs/diagnostic-enrollment-inventory-20261015/proposal.json)
are for explicit operator selection, not installation or runtime admission.

## Observed host and existing installation

The existing strict known-host SSH route is `hydracache-ax42`, host
`37.27.115.91`, account `hydracache-admin`. No host key was accepted or updated.
The proposed trust base includes this host's kernel, root administrator and
runner administration; SSH inventory is not remote physical-host attestation.

| Input | Observed value |
| --- | --- |
| Kernel | `6.8.0-138-generic`, x86_64 |
| Machine ID | `39d3868c53dc43eabec0a9abe675add4` |
| Boot ID | `e81671d6-bf23-4ebf-b8fe-5dad1f45866f` |
| Worker account | `hydracache-perf`, UID/GID `986/986`, no explicit supplementary groups |
| Local transport account | `github-runner`, UID/GID `1001/1001`, explicit client group `987` |
| Client group | `hydracache-perf-client`, GID `987`, explicit member `github-runner` |
| Installed source | `543108f1ccd206ae670803c2617f07e7fa91ae62` |
| Installed binary SHA256 | `f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62` |
| Stable supervisor | PID `8839`, active/running, `NRestarts=0` |

The installed source and binary/receipt hashes match between the two preflight
captures. All four lifecycle snapshots report absent active-campaign and fixture
markers. Prefix-scoped process enumeration finds only the stable supervisor;
it is not an exclusive host reservation or proof against a later start. The
runner is inactive locally and offline/not busy in GitHub. No queued or running
GitHub Actions workflows appear in the retained API snapshots.

The exact passwd SHA256 is
`8f9be9fe394378826b0fdbe631e08cc0d75775845386d90bef3501ab427aabda`;
group SHA256 is
`adfa40e315c04a2d85b480d54a08652e7d4cd43eb4d11ab5998c89eb40aaac4e`.
These describe local files, not NSS behavior or actual worker credentials.
The three fixed worker/controller/OIDC trust files, diagnostic-intents directory
and worker-authority journal directory are absent at their observations. No
current production policy digest or authority epoch can be read from them.

## Receiver namespace must precede the signed body

The SSH reader and PID 1 share user, mount, PID and time namespace objects. The
running supervisor shares user/PID/time but has another mount namespace:

| Namespace | SSH and PID 1 inode | Existing supervisor inode |
| --- | ---: | ---: |
| user | 4026531837 | 4026531837 |
| mount | 4026531841 | 4026532718 |
| PID | 4026531836 | 4026531836 |
| time | 4026531834 | 4026531834 |

All namespace device values are `4`. The actual service reports `PrivateTmp=yes`,
`ProtectSystem=strict`, `ProtectHome=yes` and `ProtectControlGroups=yes`.
The existing kernel-policy join compares the receiver's original current context
with its signed policy, so the SSH mount namespace is not a valid substitute.
Neither the current supervisor inode nor PID 1's inode is selected for the future
receiver. A service replacement may create another namespace object.

Preserve confinement. After an exact receiver installation is separately
authorized, capture that receiver's original context through the controlled
bootstrap, independently approve it and only then finalize/sign the policy body.
Do not remove `PrivateTmp` or relax namespace equality to make a draft pass.
Any receiver replacement must refuse old bindings and require explicit enrollment;
no automatic pin refresh or namespace adoption is allowed.

The root cgroup has neither `cgroup.type` nor `cgroup.kill` at these observations.
The actual supervisor cgroup is `domain` and has `cgroup.kill`. Root-file absence
therefore does not establish missing kernel support. Presence is not a tested
write privilege or permission to kill the supervisor. `KillMode=process` also
does not establish termination of all future writers.

## Proposed worker and authority selection

Reuse the selected worker issuer; do not generate or rotate a key. Propose the
observed worker IDs, empty explicit supplementary list and exact account-file
hashes under the existing local-files profile. Receiver namespace binding remains
unresolved, so the proposal deliberately has no worker-policy body SHA256 and
is not a valid worker-policy or trust-input document.

Propose **explicit initialization of a new diagnostic authority at epoch 1**,
not discovery of a current epoch. This choice requires operator approval of the
authority history and host trust, followed by exclusive checks of newly provisioned
empty roots. An absent path does not prove there was never earlier authority.
If previous state, ambiguity, a partial record or an active attempt exists, refuse
instead of initializing, resetting, repairing or lowering an epoch. Whole-host
rollback and compromised kernel/root remain outside the selected local model.
No directory, epoch or signature was created in this inventory.

## Proposed GitHub signing job selection

The repository API reports ID `1217101761`, public visibility, default branch
`main`, and read-only default workflow permissions. The feature branch points
to `323c2509a7d20969b8e9216340675883afa77894` and is not branch-protected.
The dedicated controller environment and registered workflow return HTTP 404;
the workflow file is also absent at that exact feature-branch revision.
An exact future workflow SHA cannot be selected from this source.

Propose the existing designed workflow ref and a **new**, separately protected
`performance-diagnostic-controller-074` environment: exact feature-branch policy,
required reviewer `javaquasar` (ID `22050429`), no administrator bypass and no
automatic approval. In the one-human-plus-agent model, `prevent_self_review=false`
permits that human to approve their own dispatch; this is one human approval,
not two independent human reviews. Every run still needs an exact reviewed
workflow SHA. A branch-name policy alone cannot protect mutable branch code.

The actual OIDC settings are `use_default=true`, `use_immutable_subject=false`,
prefix `repo:javaquasar/hydracache`. On that configuration, propose
`repo:javaquasar/hydracache:environment:performance-diagnostic-controller-074`
as the expected subject, based on GitHub's
[default environment subject format](https://docs.github.com/en/actions/reference/security/oidc).
This is a derived proposal, not a claim that an actual job token was received.
Refuse a changed subject configuration, workflow revision, event or runner policy.

The existing `performance-reference-074` has no protection rules and no deployment
branch restriction. Its secret metadata lists `HYDRACACHE_074_AUTH_SIGNING_KEY_HEX`;
the protected builder environment lists a distinct builder secret. No secret
value was retrieved, and metadata alone does not prove correspondence to a public
pin. Do not reuse the unprotected environment or silently broaden the builder
key's purpose. No protection, secret, branch or workflow setting was changed.

Keep compilation in a job without OIDC/signing authority. The future signing job
may consume only independently admitted prebuilt launcher/signer/runtime inputs.
Its receipt, original runtime root and exact workflow SHA remain unresolved.
The host's observed GNU/CA hashes are not the GitHub signing job's runtime pins.

## Proposed OIDC roster and validity

One bounded public HTTPS acquisition reproduced the October 14 discovery/JWKS
digests and four distinct RS256 keys. Propose all four exact members listed in
`proposal.json`, rather than guessing an active key. This uses the existing v2
bounded roster without changing algorithms or enabling network refresh.

Proposed snapshot validity is `2026-10-15T08:30:35Z` through the exclusive expiry
`2026-10-16T08:30:35Z`, exactly 24 hours from acquisition. Approval after expiry
requires another independently reviewed public snapshot; neither a delayed job
nor this report can extend the interval. Transport observation under the local
platform CA store is not itself production issuer/custody enrollment.

## Approval and implementation order

First approve or amend the host/kernel/administrator trust model, worker/account
and new-authority initialization proposal, protected signing-job profile and
bounded roster policy. That approval selects policy intent; it does not fill
missing executable, receiver or workflow facts with assertions.

Next implement and review the exact bootstrap/runtime and workflow artifacts,
then separately authorize protected configuration and quiescent installation.
Capture the actual receiver context, finalize canonical worker policy and its
independent body pin, and obtain the real signer-build receipt under the selected
issuer. Only then connect original receiver admission, replay/common fence,
authenticated original-start, watchdog and all-writer cessation. Completion,
cells 2–4 and their SIGKILL/lost-ack/restart matrix follow those producers.

This review does not close P2/P3 or authorize server rehearsal, a product pilot,
six-hour/24-hour qualification or release publication.
