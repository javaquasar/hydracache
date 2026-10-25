# GET owner full D3 preparation on the dedicated host

The fixed-material instrumentation closes one full-D3 prerequisite: fresh secure
processes can consume the same pinned diagnostic PKI instead of generating a new
certificate chain each time. A separate synthetic GNU control demonstrates a
possible allocator-residency measurement boundary on this host. Neither result
accepts GET-owner; the numerical full-D3 cohort has not started.

## Shared material and compatibility

Contract `get-owner-full-d3-material-contract.toml` was committed at `3c1a75e3e`
before implementation. Implementation `5a3785a6a` adds exclusive private file
creation, a 64-KiB bound, strict schema 1, raw-byte SHA256 pinning before decoding,
certificate/key consistency checks, Unix mode/link checks and two independent
reads of the same material. The receipt contains only hashes and public
certificate fingerprints. Diagnostic private keys are not retained in evidence.

`--generate-material` and `--validate-material` start no fixture.
`--run-with-material` requires the binary/source/material pins and an mTLS
surface. The normal per-socket RESP AUTH, TLS verification, byte oracles, final
dataset checks and explicit shutdown remain. The legacy `run` API, CLI grammar,
ephemeral mode and JSON envelope are preserved; the new `fixed_material` field
is absent from legacy output. A receipt still cannot prove cross-process parity
by itself: the coordinator must compare both material and dataset identities.
Windows checks do not certify Unix permissions or Windows ACL enrollment.

The first Linux material suite failed three cases and the first Windows suite
failed two. Metadata-only validation reached Rustls before a server installed a
default crypto provider; both ring and AWS-LC were present. The correction uses
an explicit ring provider only inside the new certificate validator, without
installing or changing the legacy transport's process-global provider. Original
failure logs remain alongside the corrected source overlays.

The final focused results are 89 complete standalone tests per Windows off/on
variant, six metadata-only Linux tests per variant, all-target/all-feature checks
and strict Clippy on both operating systems, plus formatting. Windows semantic
tests launch fresh RESP2, RESP3 and HC2 mTLS processes against the same pin and
exercise old modes. The Linux host checks deliberately do not start those
product fixtures before the coordinated host execution gate.

## Shipping GNU allocator capability

The host inventory identifies the installed libc as GNU 2.39 and finds
`glibc.mem.decorate_maps` with default zero. The synthetic child alone enables
mapping decoration; host environment and allocation-policy tunables stay
unchanged. GNU documents the tunable as labeling library-owned mappings, not
replacing the allocator. [GNU memory tunables](https://sourceware.org/glibc/manual/2.44/html_node/Memory-Related-Tunables.html)

The frozen control uses a small live sentinel and five snapshots: baseline,
allocate/touch 2 MiB, free, same-size refill/touch and free again. It does not call
trim or purge. Each snapshot brackets a static-buffer smaps read with mallinfo2.
In all five, the inventory remains stable and the writable allocator-map bytes
equal `arena + hblkhd`. Main-heap and large-malloc mapping labels are observed.
The first large allocation disappears on free; refill uses heap space and the
last free leaves resident/free allocator storage visible. This distinguishes a
released mapping from retained heap memory without renaming whole-process RSS.

These are synthetic provider observations, not cache memory ratios. The control
is single-threaded and does not prove multi-arena or remap coverage. A strict
observer must refuse unknown/missing coverage, malformed input and unstable
snapshots. `uordblks + hblkhd` is allocator-accounted in-use storage; `fordblks`
reports free bins/top, not every application-freed byte. Tcache accounting must
remain explicit. See the [GNU field definitions](https://sourceware.org/glibc/manual/2.39/html_node/Statistics-of-Malloc.html)
and [the corresponding allocator accounting implementation](https://github.com/bminor/glibc/blob/glibc-2.39/malloc/malloc.c).

Contract bytes and C source were frozen into the launch before this first
control; the contract was subsequently committed at `c3711a8ef`. Compilation
uses strict C warnings. No measured pair, practical-effect estimate or accepted
System-retention lane follows from this feasibility result.

## Host state and retained evidence

The initial clean-source Linux off/on build consumes source `5fc6197bf`, 2572
verified inputs and unchanged locks. Its protected images are:

| Variant | SHA256 |
| --- | --- |
| off | `8ddbbc72d5590c6e9679c846dd5b3fa8330b95833db907ce5c208275857370be` |
| on | `444ef436230211d3242bb9fcf47d13ae420af03d8f88477d79251c17efa1e4d1` |

Those images predate the material change and were not executed. They cannot be
reused as final-source full-D3 binaries. Material checks bind explicit 2573-file
working-tree overlays; their receipts do not claim a clean final commit build.

Pre/postflight keeps supervisor PID 8839, zero restarts, the same boot and PID1
mount inventory, unchanged installed files and no campaign/lifecycle markers.
No release is installed, service stopped, clock or AppArmor setting changed.
The operator has now approved a short maintenance of only
`hydracache-performance-supervisor-074.service` for fenced D3 execution with an
owned cgroup, autonomous watchdog and verified service restoration. That window
has not been consumed by these preparation checks.

The [raw packet and offline reader](local-runs/get-owner-full-d3-material-20261025/index.json)
retain 210 logical members, including both first failures, commands, source
inventories and complete logs. Five reader controls reject raw mutation,
rewritten first refusal, lost mapping coverage and promotion of preparation
into D3.

The documentation consistency and local performance-contract checks also pass;
their original transcripts are included. Existing full-workspace/security gate
failures are not relabeled by these focused results.

Replay with:

```text
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-material-20261025/test_verify.py -v
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-material-20261025/verify.py
```

## Remaining full D3 work

Complete and fault-test the shipping-GNU memory reader, including multiple
arenas and unknown/remapped mappings, then connect it to the separate
allocation/idle/delete/refill/close lane. Preserve unavailable/unstable refusal;
do not replace allocator resident memory with process VmRSS or swap in jemalloc.

Resolve fixed-work timing capacity before sealing the finite cohort. The v1
driver still has its original 10000-operation, 15-second offer-window and
60-second fixture bounds and one-second CPU/wall quality floors. This packet
does not prove that every native cell can satisfy them. Any new method needs a
separate preregistration; historical v1 inputs and the first B0 failure remain.

Finally seal the required RESP2/3, plain/matched-mTLS, sizes/transitions,
miss/error/slow-reader and concurrency cells plus all four independent native
surfaces; run five counterbalanced A/A and A/B pairs with all original effect,
noise and nonregression thresholds. The owned-tree/fence/watchdog/restoration
gate and exact-source semantic receipts precede admission. New optimization
hypotheses remain paused. GET-owner is default off; C74 and qualification
identities are unchanged.
