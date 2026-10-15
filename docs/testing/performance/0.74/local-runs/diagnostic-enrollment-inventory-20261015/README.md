# Diagnostic enrollment inventory evidence

Read-only SSH and GitHub GET observations support the
[operator review](../../diagnostic-enrollment-review-20261015.md) and the
[non-loadable proposal](proposal.json). Baseline HEAD is
`323c2509a7d20969b8e9216340675883afa77894`. The packet includes its reviewed
collector source bytes and hashes; collectors were staged locally during capture,
not run from a clean implementation commit. This is inventory, not qualification.

## Capture scope

The unchanged `performance_rental_preflight_074.py` supplies the two installation,
lifecycle and fixed two-second CPU observations. The two new packet-local
collectors read public account/context/metadata and the existing supervisor's
fixed systemd properties. Final collector source bytes were streamed directly
to SSH stdin, without uploading files, through the existing strict known-host
route and `sudo -n /usr/bin/python3 -`. No remote file, service, cgroup, namespace
or account was deliberately mutated. Ordinary SSH/sudo audit logs and access
times may change. Metadata/system utilities execute normally; inspected product
and signer inputs were not launched as workloads.

No process arguments, process environment, shadow file, real issuer private key,
signing seed or bearer token was read into this packet. GitHub authenticated
GETs used the existing local CLI credential; secret endpoints returned names and
timestamps only. Public OIDC acquisition used the existing bounded helper in a
clean owned process and requested discovery/JWKS, not an identity token.

`preflight-before.json`, `preflight-after.json`, `host-inventory.json` and
`supervisor-inventory.json` retain decoded UTF-8 stdout normalized to LF; they
are not raw SSH transport captures. GitHub retains explicit GET commands/public
projections and sanitized 404 outputs, not HTTP authentication headers. The OIDC
report retains exact public source bodies in base64. Nanosecond integers stay
in their original decimal representation; no floating-point JSON rewrite is used
for host captures. `manifest.json` hashes the retained bytes.

## Findings and proof limits

The supervisor remains active at PID 8839 with zero restarts and unchanged old
installed receipt/source/binary. Lifecycle markers are absent in the snapshots.
The runner is offline and the API snapshots contain no running/queued workflows.
These observations do not acquire an exclusive host reservation.

The SSH and supervisor mount namespaces differ. The proposal retains both and
leaves actual receiver enrollment unresolved. Trust files and authority/intent
roots are absent at read; absence cannot select a current epoch or permit replay.
The proposed new-authority epoch 1 is explicitly unapproved, not an observed value.
Missing root cgroup files do not negate the observed service cgroup's domain/kill
interfaces. No cgroup write privilege or all-writer revocation was tested.

The controller workflow/environment are absent; the existing reference environment
is unprotected. The selected signing-job/runtime/receipt/receiver inputs remain
unresolved. Proposed OIDC membership and 24-hour expiry are not installed trust.
Earlier failed collection commands and their corrections are retained in
`development-observations.json`; neither is counted as a successful capture.

## Offline verification

```sh
python -B -E -s docs/testing/performance/0.74/local-runs/diagnostic-enrollment-inventory-20261015/test_collect_host.py
python -B -E -s docs/testing/performance/0.74/local-runs/diagnostic-enrollment-inventory-20261015/verify_packet.py
python -B -E -s docs/testing/performance/0.74/local-runs/diagnostic-enrollment-inventory-20261015/test_verify_packet.py
```

Collector tests cover bounded read-only files, symlink/oversize/absence/permission
handling, local account membership, fixed safe systemd queries, generation and
unit drift, and refusal to promote observations. Packet mutations cover false
versus zero, approval flags, fabricated epoch/receipt, receiver namespace
substitution, account hashes, active campaign/restart, mutating GitHub method,
missing environment, roster substitution, snapshot extension and duplicate JSON.
These tests verify evidence consistency, not the truthfulness of an arbitrary
producer or authenticated original-start.

Frozen qualification and root lock digests remain unchanged. P2/P3, completion,
later cells, installation/rehearsal, performance and ship gates remain closed.

## Local check results

Windows executes 11 collector checks with two Unix-only cases skipped; Linux
executes all 13. Both platforms pass 12 copied-record mutation tests and the 27
existing preflight/public-source/acquisition regression tests. The latter use
synthetic loopback TLS, not another GitHub acquisition. All 152 root performance
contract tests pass. Doc-check and the local non-promotable contract pass;
require-ship refuses with the expected exit 1.

Full-workspace Windows `cargo fmt --all -- --check` reports the platform filename
or extension length limit (OS error 206). The scoped xtask and supervisor format
check passes separately, as does the 171-gate registry; no Rust source changed. This packet
does not claim a full workspace verify, release qualification or production
execution test. Exact final logs and hashes are retained in the manifest.
