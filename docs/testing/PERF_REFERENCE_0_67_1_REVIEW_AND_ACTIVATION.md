# 0.67.1 Reference Review, Activation, and Frozen-Candidate Runbook

This runbook covers W5-W7 after one 0.67.1 bootstrap campaign has produced five accepted
samples. It does not change any workload, repetition count, zero-error rule, SLO, `0.15`
scenario spread limit, calibration rule, affinity, quota, privacy, IRQ, or fail-closed check.

There are two deliberately separate campaigns:

1. the **bootstrap campaign** runs qualification, two full-dress admissions, and five serialized
   non-ship samples on one pre-activation `main` SHA;
2. the **frozen-candidate campaign** runs after reviewed contracts are committed and executes the
   complete ship gate on the new exact `main` SHA.

The frozen candidate is never a member of its own baseline.

## Authorities and immutable inputs

The workflow separates four authorities:

| Authority | Action | May claim ship eligibility? |
|---|---|---:|
| Measurement runner | Produces qualification, full-dress, and five chained samples | No |
| Proposal automation | Deterministically derives all four contract files from all five samples | No |
| Independent reviewer | Approves or rejects the exact proposal bytes and digests | No |
| Frozen-candidate workflow | Revalidates committed review and executes the complete current-SHA pipeline | Yes, only when every gate passes |

The original GitHub artifact ZIPs remain unchanged under the external campaign directory. The
controller creates a second, digest-verified view for W5; it never substitutes that view for the
originals.

## 1. Finish and seal the bootstrap campaign

Run the controller from an exact clean checkout of the pre-activation `main` SHA:

```bash
python3 scripts/perf/reference_campaign.py run \
  --campaign-dir /var/lib/hydracache-campaigns/<campaign-id>
```

After the fifth accepted sample, the controller:

- runs the Rust five-sample-set validator;
- keeps every original artifact ZIP and its byte size/SHA-256;
- creates `reference-inputs/sample-1` through `sample-5`;
- copies only evidence paths declared by each `bootstrap-sample.json`;
- rejects absolute paths, traversal, symlinks, encrypted/oversized members, duplicates, missing
  files, and digest drift;
- writes `reference-inputs/reference-inputs.json` and a byte-identical
  `reference-inputs/bootstrap-sample-set.json`.

For a campaign completed by an older controller, reconstruct the view without rerunning a sample:

```bash
python3 scripts/perf/reference_campaign.py prepare-review \
  --campaign-dir /var/lib/hydracache-campaigns/<campaign-id>
```

This command is idempotent only while every retained byte is unchanged.

## 2. Produce the deterministic W5 proposal

Use a stable automation identity, an RFC 3339 UTC timestamp, and a specific rationale:

```bash
cargo run -p xtask --locked -- perf-reference \
  --release 0.67.1 \
  --profile reference-v1 \
  --phase propose \
  --sample-set /var/lib/hydracache-campaigns/<campaign-id>/reference-inputs/bootstrap-sample-set.json \
  --samples-dir /var/lib/hydracache-campaigns/<campaign-id>/reference-inputs \
  --producer reference-automation \
  --proposed-at 2026-08-06T12:00:00Z \
  --rationale "bootstrap exact five-run reference contract"
```

The proposal is written with create-new semantics under
`target/test-evidence/0.67.1/reference-proposal/`. It contains:

- the exact five receipt digests and sample-set digest;
- the full-dress admission and runner-provisioning digests;
- the previous baseline digest;
- proposed profile, budget, baseline, and anchor digests;
- a sealed proposal receipt.

All five eligible samples participate. Medians are derived from the five-member set; the tool does
not choose the fastest run. The activated per-report spread ceiling is `0.05`; the existing
per-scenario `0.15` eligibility gate remains unchanged and is evaluated earlier.

## 3. Record an independent decision

The reviewer must inspect the original archives, sample chain, proposal receipt, numerical
payloads, claim scopes, fingerprint, method, and diffs. The reviewer must not use the proposal's
producer identity.

Create `target/test-evidence/0.67.1/reference-review-decision.json`:

```json
{
  "schema_version": 1,
  "release": "0.67.1",
  "profile": "reference-v1",
  "proposal_file_sha256": "<sha256-of-proposal.json>",
  "decision": "approve",
  "reviewer": "<independent-identity>",
  "reviewed_at": "<RFC3339-UTC>",
  "review_reference": "<durable-review-reference>",
  "reason": "<specific-review-rationale>"
}
```

Then run:

```bash
cargo run -p xtask --locked -- perf-reference \
  --release 0.67.1 --profile reference-v1 --phase review
```

Approval writes reviewed files under `target/test-evidence/0.67.1/reference-reviewed/` and a
sealed `baseline-review.json`. Rejection is a correct fail-closed result: TD-0013 remains open and
new evidence or a new proposal is required. Never widen a threshold to turn rejection into
approval.

## 4. Commit exactly the reviewed bytes

The activation change must contain these byte-exact mappings:

| Reviewed output | Canonical committed path |
|---|---|
| `profile.toml` | `docs/testing/perf-profiles/reference-v1.toml` |
| `budget.toml` | `docs/testing/perf-budgets/0.67.1/reference-v1.toml` |
| `baseline.toml` | `docs/testing/perf-baselines/0.67.1/reference-v1.toml` |
| `anchor.json` | `docs/testing/perf-anchors/0.67.1/reference-v1.json` |
| `baseline-review.json` | `docs/testing/perf-reviews/0.67.1/reference-v1.json` |

In the same reviewed activation change:

- add the scoped `docs/releases/0.67.1.md` release note;
- move TD-0013 to resolved;
- state the exact host/scenario/method scope and prohibit portability claims;
- do not edit historical 0.67 budget/baseline files.

The committed-review gate rehashes all four canonical files. The activation gate validates the
full contract, exactly one approved fingerprint, independent review provenance, five unique sample
receipt digests, TD closure, release notes, and the no-self-baseline rule.

## 5. Merge ordinary CI before renting final measurement time

Before the frozen run, the activation PR must pass the normal required checks, including Rust,
MSRV, Shared Tripwire, governance, canaries, formatting, and documentation. Merge only after those
checks are green. The exact merge SHA becomes the frozen candidate.

Provision/freeze a fresh campaign state for that SHA. The same physical server may be reused only
if its immutable identity and host contract remain unchanged; the new SHA still requires new
commit-bound provisioning and host-admission receipts.

## 6. Execute the separate frozen-candidate campaign

From the exact clean new `main` checkout:

```bash
python3 scripts/perf/reference_campaign.py run-frozen \
  --campaign-dir /var/lib/hydracache-campaigns/<frozen-candidate-campaign-id>
```

The controller keeps the runner offline except for the one serialized dispatch and rejects foreign
or ambiguous runs. The workflow executes, in order:

1. clean trusted `main` checkout and tmpfs preparation;
2. committed five-sample review revalidation;
3. activation and TD-0013 validation;
4. current-SHA provisioning import, attestation, and seven-probe preflight;
5. exact prebuild;
6. core reference evidence;
7. rootless-Docker RESP and pinned Redis comparison;
8. real 3/5/7 daemon control-plane evidence;
9. tmpfs materialization;
10. 0.67.1 budget and rolling-baseline verdict;
11. complete expected-red canary sweep;
12. sealed frozen-candidate receipt and `--require-ship` aggregation.

The controller downloads and retains the immutable artifact ZIP, validates the current SHA/run
identity, requires `ship_evidence_eligible=true`, re-hashes every receipt-bound reference/canary
member plus the activation and budget verdict from that ZIP, and requires the exact W0-W7
ship-ready aggregate. A failed, unstable, incomplete, or identity-mismatched run is retained as
diagnostic evidence and does not count.

## 7. Teardown boundary

Only after no immediate rerun or audit remains:

```bash
python3 scripts/perf/reference_campaign.py close \
  --campaign-dir /var/lib/hydracache-campaigns/<campaign-id>
```

`LOCAL_HOST_CLOSEOUT_COMPLETE=true` means only that local services are stopped and the final host
archive exists. The controller also prints `SERVER_DELETION_BLOCKED=true`: complete and verify the
off-host campaign/host-state copy and GitHub artifacts, commit the sanitized report, run the secret
scan, and revoke runner credentials before separately authorizing provider deletion and confirming
that billing stopped.

## Failure interpretation

- A W5 rejection means the numerical reference is not acceptable; collect evidence or revise the
  proposal rationale, never eligibility rules.
- A W6 rejection means committed bytes, provenance, documentation, fingerprint, or
  no-self-baseline invariants differ.
- A W7 rejection means the current candidate or current host did not reproduce the reviewed
  contract. It cannot be replaced by a bootstrap receipt.
- Hardware, kernel, topology, governor, turbo, storage, profile, prebuild, or scenario-contract
  drift starts a new qualification/fingerprint family.
