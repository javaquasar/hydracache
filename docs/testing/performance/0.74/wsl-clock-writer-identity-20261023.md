# WSL clock writer namespace identification

Read-only metadata joins the currently visible systemd-timesyncd process across
two procfs views. It does not authenticate the executable behind any kernel PID
in the earlier [setter capture](wsl-clock-setter-20261023.md). Access to the root
debug shell refused because this Windows process is not elevated. No clock or
service policy was applied, and the original-owner gate remains closed.

## Scope and immutable inputs

The metadata session began at `5099a8f9`. Offline analyzer and controls are
committed separately as `87129c0b`. The first setter capture index remains
`6b54d38f0ce7b4ceacdeb0ce879a7aee6570e53501cfc12206b39c78eaef051d`;
the offline attribution index remains
`8fd6a9410913cd24f1134bd7af5f49d16a36c0114761b191fae3c1bd5c635443`.
No capture, clock query series, original-owner retry, product workload or server
operation ran. The read-only commands did not change clocks, services, tracing
controls or the original 300/60/10-second windows.

Ubuntu and docker-desktop were already running before the metadata queries;
HydraCache074Native was stopped and was not started. The running-distro inventory
matches afterward. Boot `cf222c07-5476-4f70-9a32-455e99204d0a`, Ubuntu PID1 start
ticks `10939` and time namespace `4026531834` match the capture. The complete root
tracing-control snapshot also matches: no instances, kprobe definitions or
dynamic events remain. The existing root tracing_on value `1` was read, not
overwritten.

## Current process views

| Read context | Observer PID namespace | Selected visible identity | Relevant result |
| --- | --- | --- | --- |
| WSL system distro, root | 4026532229 | PID 193 systemd-timesyn | NSpid `193 176`; executable `/usr/lib/systemd/systemd-timesyncd` |
| Ubuntu, root | 4026532241 | PID 176 systemd-timesyn | NSpid `176`; same executable, start ticks and target namespaces |
| Ubuntu, root | 4026532241 | PID 210 agetty | Not the trace's chronyd kernel PID 210 |
| docker-desktop, root | 4026532222 | PID 1157 loki thread | NSpid `1157 1109 14`; not a valid join to trace timesyn PID 1157 |

System-distro PID 193 and Ubuntu PID 176 have matching start ticks `11340`,
UID values `996 996 996 996`, cgroup
`/system.slice/systemd-timesyncd.service`, PID namespace `4026532241`, mount
namespace `4026532373` and time namespace `4026531834`. Their executable digest
is `5e87427803daf1dbe974cc62fba335c96ff24e37892f67d9095d6703d2c32009`.
The nested NSpid suffix joins the two current views. Both root os-release reads
identify Ubuntu 24.04.1 LTS.

These are sequential post-capture reads with equal before/after PID, comm and
start ticks, not a held-pidfd proof or an authenticated historical join. The trace
did not retain an NSpid chain, executable digest or start ticks for kernel PID
1157. A matching comm and shared boot are insufficient to supply those missing
edges. No leader named chronyd was found in the three bounded procfs
enumerations; that does not establish its absence from the VM root namespace.
Likewise, the current `/init` executable in system and Docker views does not
identify the trace's earlier init tasks.

## Access and tooling refusals

All first results are retained, including failures:

- The default-user system-distro multi-path readlink exited 1 after reporting
  only the observer namespaces. An explicitly root query subsequently returned
  `/init` and PID1's namespace; the earlier result was not overwritten.
- Docker's BusyBox readlink accepts one FILE, so the initial multi-path queries
  exited 1 with usage text. The later per-path shell query is a separate command.
- Docker `/proc/210` did not exist; the lookup and its exit 1 remain recorded.
- `wsl.exe --debug-shell` exited 1 with “requires running wsl.exe as Administrator”.
  It was not retried or elevated. The supplied read-only stdin script never ran.

Microsoft documents the debug shell as the way to access WSL processes outside
distribution namespaces in its [root namespace debugging guide](https://github.com/microsoft/WSL/blob/master/doc/docs/debugging.md#root-namespace-debugging).
No debugger, package installation, nsenter, mount, service action or alternate
privilege bypass was used. A further root-namespace diagnostic requires an
elevated Windows execution context, not a relaxed runtime predicate.

## Reproduction and controls

The offline analyzer checks the nested NSpid chain, current process tuple,
credentials, executable digest, cgroup and namespace links. A different boot,
identity drift, failed/duplicate fields, archive corruption, unsafe object path
or admitting scope refuses. It never turns the current join into a historical
trace identity.

The first seven-control Windows suite exposed a CRLF parsing error in exact-byte
verification. That failed log is immutable. A separate decoder correction
normalizes line endings in memory without rewriting the archive; its dedicated
regression and all eight identity controls pass. The full Linux suite passes
46 controls, including the existing mock/owned-process guardian tests. No live
tracing is enabled by these tests. Doc-check, the local non-promotable 0.74
performance contract and verification of the original setter archive pass.

The new [76-file exact archive](local-runs/wsl-clock-writer-identity-20261023/index.json)
has index SHA-256
`4d2c26ffc6219eab7664de0e77d1b072392cf4e96434a6d8343cb1a0ce76fb7c`.
It retains raw stdout/stderr, argv, exit results, supplied scripts, first control
failure, corrected controls, postflight comparisons and offline sources.

```powershell
python -B tools/wsl-clock-trace-074/identity.py verify
python -B -m unittest discover -s tools/wsl-clock-trace-074 -p test_identity.py -v
```

Retention is already consumed; neither command recollects metadata. Runtime,
tests of the supervisor, Cargo.lock and COMPAT remain unchanged against
`8fb0fb5a`; the qualification manifest retains SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

## Remaining boundary

The next diagnostic is a narrowly scoped read-only root-namespace lookup from an
elevated Windows context, preserving this access refusal and the first capture.
It must still distinguish currently surviving tasks from historical identities.
An agreed clock policy must account for all relevant writers sharing the time
namespace; this metadata does not justify disabling only the visible Ubuntu
service or changing clocks to pass the original-owner gate. Policy changes,
new captures and original-owner execution require separate scope.
