# Pure local context document allocation baseline

This package freezes the existing zeroed Vec allocation and a pure read-body
model before any runtime replacement. It uses the existing opt-in allocator
counter; production worker context, validators and deadlines are unchanged.
Only a private test module hook is added to an old supervisor source file.

`check.ps1 -Lane host|194|188 -Cohort initial|corrected` refuses existing logs.
Linux lanes run five pure controls and two existing allocator-counter controls,
then affected all-target check and strict lint. Windows runs source canaries
and affected check/lint. No ignored test, Probe, original owner, proc read,
clock observation, server, service operation or product workload is invoked by
the controls. WSL is used only as an ordinary-user Rust build/unit environment.

The first Linux compilation failures and original test/recipe sources are
retained. Corrected controls fix only a json-macro expression and a usize
annotation. They do not implement a runtime candidate or replace an outcome.

After sealing, review hashes, counters and source identity without Rust/WSL:

```powershell
pwsh -NoProfile -File tools/context-document-baseline-074/review.ps1 -Mode Review
```

Requested bytes are Rust allocator requests, not usable-size, retained memory,
RSS, CPU, read latency or ReservedLive improvement. A future short stack helper
needs its own isolated implementation and evidence; baseline success opens no
material, cleanup, delivery, fault or server admission gate.
