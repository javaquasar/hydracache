# Nonseeking probe registry writer controls

The collector now sends registry commands without constructing a Python append
stream. The local controls pass; real tracefs registration has not been tested
with this candidate. Both earlier live refusals remain unchanged.

## Isolated correction

`capture.py` now opens the existing registry using
O_WRONLY|O_APPEND|O_CLOEXEC, without O_CREAT, O_TRUNC, fdopen or lseek. It
encodes one nonempty ASCII command plus its newline, calls os.write once and
requires the full byte count. Short/zero writes raise EIO; the collector does
not resubmit a command or append a guessed remainder. As documented for
[os.write](https://docs.python.org/3.12/library/os.html#os.write), Python can
internally restart an interrupted syscall if the signal handler does not raise;
the collector adds no retry loop. Empty, multiline and non-ASCII commands refuse
before opening. A write error does not prove the command was unapplied: existing
registry inspection and exact-definition cleanup checks still decide retirement.

The acquired FD is closed once on success or write failure. Open refusal does
not close an unowned descriptor. A close error after a write error is retained
as a note rather than replacing the first failure; close-only refusal remains
explicit. Errors keep the exact path. Non-append control writes are unchanged.

Both registration and exact owned deletion use this same path. Preflight,
probe definitions/filter, cleanup selection, guardian, live validation and all
original deadlines are unchanged. This correction does not authorize replay of
either consumed diagnostic lane.

## Baseline and first results

Before implementation, the new regression control rejected the old high-level
append path with the expected exit 1. Its source snapshot and failure log are
retained. The first full candidate suite passed the nine new writer tests but
failed an existing guardian SIGKILL control: its harness read a visible but still
empty receipt and raised JSONDecodeError. That first 30-test outcome is retained
as exit 1, not replaced by the later result.

A separate test-only correction adds a completion pipe after the real receipt
write/flush/fsync. The harness keeps its four-second wait limit, parses JSON once
and never polls until parsing succeeds. An additional deterministic control
creates a visible empty receipt before completion, proving visibility alone
cannot admit a read. Production receipt publication and guardian code are
unchanged. With this explicit method correction, all 31 controls pass.

The nine writer tests cover exact syscall order/flags and no seek; all seven
registration and seven deletion payloads; short/zero writes; open, write,
interrupt and close errors; preservation of the first error; invalid commands;
and append/prefix preservation plus missing-file refusal on owned regular files.
They never use live tracefs. The previous ABI, cleanup, configuration, offline
decoder and owned-process guardian controls are also retained.

## Evidence and gates

Writer commit: `e95bf69c`. Separate harness commit: `7f4f2293`.
[Exact control archive](local-runs/wsl-clock-writer-20261023/index.json):
37 logical files, 30 objects, SHA256
`de758b208afe099e48da3f2fc176382a25b26fa9887c46b9a4ed4daa4aebec07`.
The archive declares zero tracefs writes and no tracing-state effects. It retains
the old writer, regression source, first failed suite, corrected harness result,
candidate sources and gate logs. The common exact-object reader verifies it:

```powershell
python -B -c "import sys; from pathlib import Path; sys.path.insert(0,'tools/wsl-clock-readonly-074'); import offline_join; f=offline_join.load(Path('docs/testing/performance/0.74/local-runs/wsl-clock-writer-20261023'),'de758b208afe099e48da3f2fc176382a25b26fa9887c46b9a4ed4daa4aebec07'); print(len(f))"
wsl.exe -d Ubuntu -u artur --exec python3 -B -m unittest discover -s tools/wsl-clock-trace-074 -v
```

Doc-check, the local non-promotable performance contract, both older trace
archive verifiers, frozen input digests and runtime diff guard pass. No runtime
crate, runtime test, Cargo.lock, COMPAT or qualification manifest changed.
No clocks/services were changed, original-owner or product workload executed,
packages installed or server invoked. There is no performance measurement or
clock-setter attribution in this step.

The next gate is a separately authorized fresh configuration-only lane: verify
the corrected registration/deletion path with the private trace buffer disabled,
the same preflight, guardian deadline and cleanup/context proof. A clock capture
and the original-owner/server gates remain separate subsequent work.
