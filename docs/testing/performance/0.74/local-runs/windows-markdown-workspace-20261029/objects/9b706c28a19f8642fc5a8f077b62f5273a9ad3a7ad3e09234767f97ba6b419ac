# Local static contract Git attribution

This Windows-only diagnostic observes one unchanged, already built
`performance_contract_074` executable. It is not the full package, a product
candidate, original-owner control or server runner. No Rust source or assertion
is instrumented or replaced. Git's own Trace2 writes one JSON file per process;
the parent records monotonic wall time and `GetProcessTimes` through Popen's
retained Windows process handle. Parent CPU excludes children.

Default invocation only prints the prepared scope. Execution requires the
explicit switch and exclusively creates the fixed first-attempt directory:

```powershell
python tools/xtask-git-trace-074/capture.py
python tools/xtask-git-trace-074/capture.py --run-once
```

The required no-run build receipt is
`target/source-continuation-20261024/static-trace-build`. Preflight requires a
fresh Rust 1.94 test artifact and unchanged captured inputs. Only this new
Python/README diagnostic directory may have been added after the build. The
expected 321 case names come from the hash-verified retained package evidence;
the executable's `--list` output must match every name. The actual invocation
uses four case threads, no filters, ignored tests or nocapture mode.

Before the target, a read-only `git --version` control must create one complete
Trace2 version session in a separate directory. Existing Trace2 configuration,
capture-affecting environment or Git replacement refs refuse. No Git config,
PATH, executable, process priority, clock or service is changed. Trace2 variables
are supplied only to the diagnostic child environment, not to metadata queries.

The target bound stays 780 seconds and the disk floor is 4 GiB. Source, HEAD,
binary and exact case results are checked. Timeout/exception cleanup addresses
only the owned test process tree; the fallback uses Popen's exact handle, not an
image-name lookup. Cleanup is not production all-writer retirement proof. A
first refusal or partial trace is retained and not retried to pass.

```powershell
python -m unittest discover -s tools/xtask-git-trace-074 -p test_*.py
python -m compileall -q tools/xtask-git-trace-074
python -m tabnanny tools/xtask-git-trace-074
```

Controls cover exact catalogs, case/exit/format drift, truncated or duplicate
sessions, lifecycle ordering, nonfinite numbers, current-process handle identity
and mocked exception cleanup. Trace2 format 4 is required; missing shutdown,
discard sentinels or mixed SIDs refuse. Nonzero Git exits remain data, since
negative source controls may intentionally observe them.

Summed Git elapsed spans can overlap under four case threads and omit Windows
process creation before Git's trace initialization. They cannot be added to or
subtracted from target wall time as CPU, I/O or parser cost. Repeated full-hash
`show` argument shapes identify a follow-up lookup hypothesis, not a cache
admission: short revisions, dynamic refs and current-file reads are not folded
into that count. Rust parsing/token-comparison time and Trace2 observer overhead
remain unattributed. One instrumented observation is not a speedup estimate.
