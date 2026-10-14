# Reuse the reviewed diagnostic public keys

The controller and distinct builder public keys already have a reviewed
[enrollment packet](local-runs/diagnostic-builder-enrollment-20261009/manifest.json)
and a [signed hosted build](local-runs/diagnostic-builder-hosted-37923423889/manifest.json).
Baseline is `194585b89b119750589e39138644a75c2b1b3cbc`. Reuse those independent
pins rather than generating replacements or letting an incoming policy choose
its own trust root.

## Fixed local verification entry points

Add a portable reviewed-trust module with the exact controller public key,
builder public key and raw builder policy SHA-256 from the prior enrollment.
Its builder loader accepts bounded policy bytes but no caller key or digest;
the existing canonical verifier checks them against the fixed reviewed pins.
Do not embed a private seed, generate keys, register secrets or retrieve secrets.

Add a separate opt-in KernelPacket authentication entry that first requires the
fixed repository and reviewed controller key in the supplied principal policy,
then uses the existing original sender and kernel-time checks. Wrong trust
permanently refuses the original packet, including attempts to fall back to the
portable supplied-key method. Principal allowlists remain independent caller
configuration, not claimed as enrolled. The existing generic method remains a
local model primitive; no listener or command-line route invokes the new entry.

These fixed source pins are the reviewed October 9 selection, not a proof of
current external revocation status, controller provenance, root-owned installed
configuration or production enrollment. Future authority rotation needs an
explicit reviewed change; a downloaded envelope cannot update them.

## Checks before implementation

Retain diagnostic artifact and IPC regression plus kernel baseline first. Add
a missing-module/API red test. Verify the retained real hosted receipt signature
under the reviewed builder key without running its executable or pretending
to re-audit absent binary bytes. Reject one-bit signature changes even with a
recomputed receipt digest, altered policy keys/repository/schema, empty/oversized
or noncanonical policy, and attempts to use controller as builder. Test fixed
controller/repository checks, independent concurrent loaders and actual local
kernel packet sticky refusal after wrong reviewed trust or a fixture signature.
Run three focused repeats, current/MSRV checks, strict lint, regression, format,
documentation and non-promotable performance contracts.

## Worker trust remains a separate input

The worker-policy packet uses synthetic issuer keys and explicitly records
`real_issuer_key_enrollment = false`. No real worker issuer, signed production
policy/body pin or current authority epoch is selected here. Do not silently
reuse the controller/builder as worker issuer or choose epoch 1. Kernel host facts
and historical host inventory cannot supply independent authority pins.

Production trust loading and host/epoch/fence joins, original start, root collector,
watchdog, every-writer drain, completion and cells two through four remain open
under the [pre-server checklist](pre-server-closure-checklist-20261014.md).
This slice changes no wire/durable schema, qualification digest, product candidate
or threshold and performs no server operation or performance measurement.
