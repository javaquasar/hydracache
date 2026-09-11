# AX42 dedicated reference campaign for 0.67.1

Status: complete dedicated-host evidence. The final frozen candidate passed and the immutable
campaign was closed on 2026-09-11. This report is intentionally anonymized and does not contain
network addresses, credentials, host keys, MAC addresses, UUIDs, device serial numbers, or
provider account identifiers.

## Scope and identity

| Field | Value |
| --- | --- |
| Release evidence | `0.67.1` |
| Reference profile | `reference-v1` |
| Final campaign | `hc0671-ax42-20260911-da` |
| Exact candidate commit | `7bd31af9a5092466d7a7284995f388d33ed3110f` |
| GitHub Actions run | `34580969962` |
| Runner class | protected, non-virtualized bare metal |
| Final phase | `closed` |
| Release aggregation | W0-W7 `ship-ready`; no rejection reasons |

The numbers below apply only to this physical host, source commit, scenario set, toolchain,
profile, and same-box method. They are not portable sizing guidance and do not establish
distributed RESP capacity or a universal Redis comparison.

## Sanitized host inventory

| Component | Reviewed state |
| --- | --- |
| CPU | AMD Ryzen 7 PRO 8700GE; one socket; eight physical cores; SMT disabled; CPUs `0-7` online |
| Cache | 256 KiB L1d, 256 KiB L1i, 8 MiB L2, 16 MiB L3 |
| RAM | 64 GiB installed class; 66,490,486,784 bytes (61.9 GiB) available to Linux |
| Storage | Two 512,110,190,592-byte NVMe devices; rotational flag false |
| RAID | Linux software RAID1; three arrays; every array healthy with both members active (`[UU]`) |
| Network | Intel I210 Gigabit controller; 1,000 Mb/s, full duplex, link detected |
| Operating system | Ubuntu 24.04.4 LTS, x86-64 |
| Kernel | `6.8.0-138-generic` |
| Virtualization | none detected |
| Cgroups | v2; CPU quota unlimited |

The RAID1 usable regions observed during closeout were approximately 31.0 GiB, 1.0 GiB, and
444.6 GiB. Device model strings and hardware identifiers are deliberately omitted.

## CPU and IRQ policy

- Measurement CPUs: `1-4`.
- Housekeeping CPUs: `0,5-7`.
- Kernel isolation: `isolcpus`, `nohz_full`, and `rcu_nocbs` cover `1-4`.
- SMT is disabled; the CPU governor is `performance`; AMD P-state turbo remains enabled under the
  reviewed maximum-frequency contract.
- IRQ affinity is housekeeping-only. Managed NVMe vectors whose effective affinity cannot be
  changed are accepted only while unmapped by blk-mq and at a zero cumulative count.
- Both measurement and housekeeping sets use the reviewed 1 microsecond maximum-idle-latency
  policy.
- The final campaign passed the 900-second IRQ burn-in and every pre/post runtime IRQ delta guard.

## Qualification, full-dress, and bootstrap

The acquisition chain remained fail-closed and immutable:

1. The physical host passed topology, isolation, NVMe, kernel, toolchain, calibration, and IRQ
   qualification.
2. Two separately dispatched full-dress runs passed and established admission for bootstrap.
3. Exactly five chained bootstrap samples from product source
   `0b6acb370ebe3e6d107a007726bcf98a4cf60ce7` were retained. All five were stable, zero-error,
   artifact-bound, from one runner fingerprint, and linked to their predecessor receipt.
4. An independent review approved the median-derived immutable anchor, rolling baseline, and
   numerical budgets on 2026-09-10. The reviewed sample-set digest is
   `e753b91818eb448f1491aaa66a87da5d8fb4bb14a15e98f47c7e47666cf8e6db`.
5. The reviewed contract was activated without admitting the candidate into its own baseline.
6. The separate final candidate ran from exact commit
   `7bd31af9a5092466d7a7284995f388d33ed3110f` and consumed the already reviewed baseline.

Rejected campaigns were retained as diagnostics and were never promoted. In particular, campaign
CY exposed a real one-of-five RESP p99 excursion at the 50,000 operations/s knee, and campaign CZ
completed the long measurements but correctly failed its late workspace receipt because two
required tools were absent. The final candidate preflighted those tools before starting long
measurements.

## Final frozen-candidate result

The final campaign passed the complete ordered pipeline:

- runner attestation, seven-probe preflight, exact prebuild, and binary/digest binding;
- embedded local cache, client surface, and node-local RESP capacity/latency evidence;
- same-box pinned Redis comparison;
- real 3/5/7-daemon control-plane evidence;
- grid-model, brownout, and overload evidence;
- immutable anchor and rolling-baseline budget validation;
- expected-red canary sweep, exact receipts, artifact integrity, and release aggregation;
- all 3,025 workspace tests used by the final receipt.

All 19 numerical budget checks passed. Selected candidate values and their effective boundaries
are shown below; floors use `candidate >= boundary`, while ceilings use `candidate <= boundary`.

| Check | Candidate | Boundary | Result |
| --- | ---: | ---: | --- |
| Embedded capacity floor | 20,000 ops/s | 18,000 ops/s | pass |
| Embedded p99 ceiling | 1,917 us | 2,137.3 us | pass |
| Client-surface capacity floor | 24,989.1 ops/s | 22,490.6 ops/s | pass |
| Client-surface p99 ceiling | 2,085 us | 2,269.3 us | pass |
| Node RESP capacity floor | 49,948.1 ops/s | 44,954.9 ops/s | pass |
| Node RESP p99 ceiling | 3,769 us | 3,890.7 us | pass |
| 3-daemon control-plane event ceiling | 848.6 ms | 1,262.1 ms | pass |
| 5-daemon control-plane event ceiling | 850.7 ms | 905.1 ms | pass |
| 7-daemon control-plane event ceiling | 903.5 ms | 932.7 ms | pass |
| Grid primitive cost ceiling | 1,253.3 ns/op | 1,405.2 ns/op | pass |
| RESP overload goodput floor | 44,374.5 ops/s | 30,153.9 ops/s | pass |

The same-box Redis comparison was stable in alternating execution order:

| Operation | Pipeline | HydraCache median | Redis median | HydraCache / Redis |
| --- | ---: | ---: | ---: | ---: |
| GET | 1 | 59,630 req/s | 87,796 req/s | 67.9% |
| SET | 1 | 59,382 req/s | 87,719 req/s | 67.6% |
| GET | 10 | 136,426 req/s | 757,576 req/s | 18.1% |
| SET | 10 | 132,979 req/s | 746,269 req/s | 18.0% |

This identifies pipelined RESP handling as the clearest measured performance gap. It does not imply
that either system was measured as a distributed cache or under a production network topology.

## Rescue and operational incidents

The first attempt to suppress managed NVMe MSI-X queues added global `pci=nomsi`. Advertising a
legacy PCI interrupt pin did not prove that an NVMe-root system could boot without MSI/MSI-X, and
the installed system did not return after reboot. Provider Rescue was used to mount the existing
RAID filesystems, remove only the rejected kernel argument, regenerate the boot configuration, and
return to the installed OS. No reinstall or evidence promotion occurred. The unsafe change was
reverted and replaced by the dormant-unmapped managed-IRQ contract.

A later burn-in found that launching a cold executable after moving it to a measurement CPU could
page code from NVMe and fire one otherwise dormant vector. Network and workload binaries are now
prefaulted on housekeeping CPUs, all external host probes run on housekeeping, and any positive
measurement-CPU NVMe delta rejects the boot.

Qualification also exposed an AMD P-state probe mismatch between the shell audit and Rust
fingerprint. Both now require the same CPB capability, `amd-pstate-epp` driver, and consistent
maximum-frequency proof. Other orchestration corrections covered bounded sudo cleanup, exact
runner labels, stale provisioning receipts, finite measurement windows, resumable artifact
retrieval, stdout draining, and early workspace-tool preflight. None relaxed an SLO, repeat count,
zero-error rule, spread limit, or evidence identity check.

## Artifact retention and integrity

The complete campaign directory was copied to operator-controlled storage outside the rented
server and outside the Git worktree before closeout. The initial remote/local comparison covered
27 files with zero missing files, zero extras, and zero SHA-256 mismatches. Extracted verification
material was added only on the off-host copy.

- Original GitHub artifact ID: `10261605556`.
- Original artifact size: 500,282 bytes.
- Original artifact SHA-256:
  `c5d4f18e037251ba66faac050460c47126d57cd42e916e2c9fa97d444df4ae2a`.
- Artifact identity matched run `34580969962` and the exact candidate commit.
- The ZIP opened successfully; 201 extracted files were readable; all JSON and XML documents
  parsed successfully.
- The after-freeze and final host-state archives each listed 14 readable entries; the admission
  archive listed seven.
- Frozen-candidate receipt SHA-256:
  `0e673a0660e68330e66bebfb664542eb35f2f2e4c34b83ca23b5fd668c326de4`.
- Final aggregate SHA-256:
  `22a4b57e791558412d7e397d9226c69e52fbb21c4f2e73183d7ef17cca146eb5`.

Raw campaign artifacts and host-state archives are not committed because they include operational
host details. Git retains only reviewed, anonymized conclusions and digest-bound evidence
identifiers.

## Conclusion

The dedicated AX42 evidence chain is complete: qualification, two-run full-dress admission, five
bootstrap samples, independent review and activation, 900-second IRQ burn-in, final frozen
candidate, artifact preservation, and release aggregation all passed. W0-W7 are `ship-ready` for
the exact `0.67.1` evidence scope. The principal optimization opportunity revealed by the campaign
is pipelined RESP processing; this conclusion is characterization, not a Redis-replacement claim.
