# Fixed controller principal and original request evidence

Implementation `b59b717f5712bff43633532d4c59cf5f246f30b6` follows preregistration
`99305011` against baseline `cbbae6499c6eccc902df99a8d1bc103c013a4f85`.
The [design](../../diagnostic-controller-trust-input-design.md) and
[contract](../../diagnostic-controller-trust-input-contract.toml) keep real
principal/host/worker enrollment, provenance/replay/fence and execution closed.

## Independent source and original ownership

The new fixed root-owned controller configuration has its own strict canonical
schema, 2048-byte limit, reviewed key/repository and required sorted unique actor
and nonroot UID/GID allowlists of at most 32 entries per list. It retains original
no-follow root/directories/file metadata and exact bytes using the existing
private bounded reader. No public path/key/principal override, writer or repair
is added. Actual configuration and allowlists are not selected or installed.

SignedControllerInput keeps mutable borrows of that source and the same original
worker/kernel guard, plus the original request and its hash. Original config and
worker checks bracket kernel signature/peer/boot/freshness verification. Later
checks require the same request and original inputs; errors latch controller,
worker policy/config/journal and packet. Healthy drop is only observation, not
nonce consumption, renewal or executor permission. No request/policy/FD extractor,
start or completion method exists. The signer's GitHub/build claims are not
independently verified provenance by this guard.

## Local gates and retained negative result

Baseline passes five worker/kernel cases, eighteen kernel cases with one manual
proof ignored, and eight reviewed-trust cases. Missing controller_input gives
expected E0432/native 101. First functional run passes six of seven: the test
restoration tries to rewrite its own mode-0400 fixture and receives EACCES.
Restoring only test permissions before rewriting fixes it without relaxing the
production metadata check. The negative raw result remains in this packet.

Final eight controller cases pass in three repeats. Seed `0x07532074` checks
32 signed disallowed-actor mutations and four independent bindings per repeat.
Coverage includes strict schema/lists/key/budget, shared no-follow/mode/link/owner
rules, original signed roundtrip and healthy drop, actor/UID/GID/repository/key/
signature/time/boot refusal, content/name/mode drift, request drift, constructor
and later refusal and permanent cross-latching after restoration. Actual public
constructor refuses local nonroot callers before accessing production files.

Linux regression passes original trust/worker/kernel and portable signature/API
targets. The registered API target now passes two tests; 73 doctests include five
new no-start/extraction/replacement/thread/borrow-refresh failures. Windows passes
22 portable cases; root suites pass 6/127/13. Current check/strict Clippy and MSRV
check pass on both platforms, Linux xtask strict lint passes, and Linux Rustdoc
warnings are denied. Linux MSRV Clippy is unavailable and not claimed. Formatting,
docs, gated registry, local performance contract, links/book pass; require-ship
refuses with expected native exit 1.

## Integrity and next boundary

The [manifest](manifest.json) pins source, raw captures, frozen qualification and
unchanged key/authority/protocol/product inputs. Verify read-only with:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-controller-trust-input-20261014/verify_packet.py
```

Positive fixtures use synthetic signing keys, private nonroot config/journal and
actual local kernel peers/context. No real private key, SSH, service, installer,
workflow dispatch or product workload is used. No new SIGKILL or full milestone
proof is claimed for this read-only step. Real allowlists/worker policy/epoch,
source-opener/physical-host and authority enrollment, independent controller/build
provenance and original durable replay/fence remain P2; authenticated collector
start, watchdog/whole-writer cessation and completion/later cells remain P3/P1.
