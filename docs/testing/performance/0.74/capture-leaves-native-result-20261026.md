# First native held owner and decode leaf control

The leaf-enabled diagnostic passes its first bounded local Windows control:
sixteen workers, sixty-four ordinary Git calls, complete outer and leaf clocks,
no operational/telemetry refusals and no cleanup errors. This establishes
functional compatibility for this exact build, not low overhead or a product
improvement. The full package and production gates remain open.

## Exact source and original execution contract

Source/HEAD is `ef3d0cadd70e436b75be94e579a605447aaf7e2b`. A new offline,
locked Rust 1.94 no-run build precedes the sole attempt. Source inventory and
HEAD remain unchanged through build/run postflight. Worker image SHA256 is
`c0e2478e96d31e1b9d2100d0bce4706720f274b68e8cda698b9dcc6780dba94f`.
The Rust worker bytes are unchanged; the build receipt additionally binds the
exact current Python sources and caller.

Only `--coordinator-frames --capture-leaves` selects this separate exclusive
attempt. The [fixed model-stage protocol](../../../../tools/xtask-package-attribution-074/capture-leaves-protocol.toml)
is retained unchanged, with its original historical model-only scope; this
packet separately records the first native execution. Its four alternating
blocks retain all original argv, environment, stdin/pipe semantics, byte/status,
pins, source/image, lifetime and terminal checks. Every Job has nine total
members, zero active members and zero forced terminations. No incomplete worker
or successful subset is substituted.

The common 120-second origin, five-second owned cleanup, 4-GiB free-space and
16-MiB log limits remain. All 256 leaf probes and 192 outer probes are complete:
448 recorder probes and 1344 recorder clock reads. Original metric/deadline
clock observations are additional and unchanged. No retry, V6, server, service,
clock adjustment, dependency change or product workload occurs.

## Retained leaf observations

The independent reader validates exact schedule/domain/raw integers, within-
and cross-group clock monotonicity, every span's arithmetic and physical
wall/CPU containment within the original spawn/decode parent. Across sixteen
workers, recorded leaf observations are:

| Leaf | Sum wall lower to upper bounds ms | Sum measured CPU ms |
| --- | ---: | ---: |
| Owner identity and path | 12.6075 to 12.8563 | 0 |
| Owner initial observation | 1.0377 to 1.0949 | 0 |
| Log stat and bound | 8.7284 to 8.8923 | 0 |
| Log read and rows entry | 380.6253 to 380.8455 | 0 |
| Text decode | 0.1209 to 0.1753 | 0 |
| Record parse | 1.7270 to 1.7830 | 0 |
| Protocol validation | 0.8042 to 0.8469 | 0 |
| Result join and residual | 0.2494 to 0.2892 | 0 |

Positive decode wall is concentrated in the read/entry bracket in this input,
not in the recorded JSON parse bracket. The maximum read bracket is
54.0593–54.0723 ms at block 2/slot 2, for a 2658-byte profile worker log.
Reference logs are 1264 bytes; profile logs span 2641–2662 bytes. Read includes
open/read/close and the call boundary, with possible scheduling/bookend costs;
it is not a pure disk or security-software cost measurement. All leaf CPU
deltas are zero, not proof that these operations perform no work or that timer
precision is known.

Original whole-frame wall sums to 21239.6077 ms and coordinator CPU to
2390.625 ms. Spawn parent wall sums to 3099.2937–3099.3792 ms with 2375 ms
measured CPU. Subtracting the two recorded owner leaves leaves an arithmetic
complement of 3085.3425–3085.7340 ms wall and 2375 ms CPU. That locates the
recorded difference outside those two leaf brackets; it does not isolate
CreateProcessW, preparation, publication or resume as a causal CPU owner.
Decode parent wall sums to 394.3889–394.4472 ms; its complement after all six
leaves is 1.5567–2.1920 ms wall, with zero measured CPU. These complements are
not extra measured phases, and nested scopes cannot be added together.

Both arms enable coordinator and leaf tracing. Neither their block difference
nor a comparison with earlier attempts measures recorder on/off overhead.
Original signed block arithmetic stays intact, with no trimming or acceptance
threshold. `overhead_accepted`, `speedup_established` and `package_pass` remain
false. `wait_with_output` still combines wait and pipe collection unchanged.

## Evidence and checks

The [164-member packet](local-runs/capture-leaves-native-20261026/README.md), in
161 addressed objects, retains two source-preserving build/run receipts, raw
intent/build/result, all sixteen worker logs/ledgers, exact current sources,
model predecessor index and the original independent outer verifier source.
Index SHA256:
`926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855`.
Image SHA256 is a retained build/run observation, not offline attestation of a
currently mapped executable.

Six integrity controls pass, including byte corruption, missing objects,
domains/order/types, partial groups, span arithmetic, parent containment,
explicit leaf intent and preserved operational verdict. The independent
verifier recomputes outer and leaf arithmetic without importing retained code,
reading clocks, invoking Git/Cargo or starting a worker. Further local check
receipts are retained separately without rewriting this first native outcome.

All 74 model/caller/accounting regressions pass again. Verifier syntax,
xtask all-target check, strict all-target/all-feature Clippy, doc-check and the
local non-promotable performance contract also pass. The
[eight-receipt final ledger](local-runs/capture-leaves-native-20261026/final-checks/README.md)
has index SHA256
`7ec1c4af34c9bfe99890d6febbe7d54ad9e2ec704bedd5c44f35cd40b19d8cab`.

Next is offline analysis of the saved read and remaining spawn observations,
followed by a separately reviewed overhead/cause-isolation method if needed.
No cache, direct native bypass, predicate removal or timeout change is justified
by these single-attempt observations. This is not Linux/MSRV/server admission,
full package coverage, a frozen C74 or production readiness. The unchanged
qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
