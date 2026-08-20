# Memory provider protocol

Each adapter implements `probe`, `start`, `mark`, `snapshot`, `stop`, and
`normalize`. Provider state and samples contain only aggregate byte counts,
fixed phase names, and sanitized folded stack names. A run is invalid unless
all eight phases occur exactly once and in canonical order.

Example:

```text
python scripts/perf/memory-providers/system.py probe --output probe.json
python scripts/perf/memory-providers/system.py start --state state.json --raw raw.jsonl --pid 1234 --binary ./hydracache-loadgen
python scripts/perf/memory-providers/system.py mark --state state.json --phase cold
python scripts/perf/memory-providers/system.py snapshot --state state.json --phase cold
```
