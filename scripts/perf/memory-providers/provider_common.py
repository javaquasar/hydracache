#!/usr/bin/env python3
"""Bounded phase provider protocol for HydraCache 0.71 memory evidence."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import sys
import hashlib
import time
from typing import Any

PHASES = [
    "cold",
    "fill",
    "steady",
    "expire_or_delete",
    "reset",
    "refill",
    "post_idle",
    "shutdown",
]
SAFE_STACK = re.compile(r"^[A-Za-z0-9_:.<>/\-+\[\] ]+$")


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def append_jsonl(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(value, sort_keys=True) + "\n")


def capability(provider: str) -> tuple[bool, str | None]:
    if provider == "system":
        return True, None
    if provider == "jemalloc":
        available = shutil.which("jeprof") is not None or os.environ.get("MALLOC_CONF") is not None
        return available, None if available else "jeprof/MALLOC_CONF is unavailable"
    if provider == "mimalloc":
        available = os.environ.get("MIMALLOC_SHOW_STATS") is not None
        return available, None if available else "MIMALLOC_SHOW_STATS is unavailable"
    return False, f"unsupported provider: {provider}"


def resident_bytes(pid: int) -> int:
    statm = Path(f"/proc/{pid}/statm")
    if statm.exists():
        pages = int(statm.read_text(encoding="ascii").split()[1])
        return pages * int(os.sysconf("SC_PAGE_SIZE"))
    if pid == os.getpid():
        try:
            import resource

            rss = int(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
            return rss if sys.platform == "darwin" else rss * 1024
        except (ImportError, AttributeError):
            pass
    raise RuntimeError("resident bytes unavailable for this pid/host")


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def validate_phase(value: str) -> None:
    if value not in PHASES:
        raise ValueError(f"unsupported phase: {value}")


def normalize(raw_path: Path, timeline_path: Path, output: Path, provider: str) -> None:
    timeline = [json.loads(line) for line in timeline_path.read_text(encoding="utf-8").splitlines() if line]
    timeline_phases = [record.get("phase") for record in timeline]
    if timeline_phases != PHASES:
        raise ValueError("timeline is missing or reorders mandatory phases")
    raw = [json.loads(line) for line in raw_path.read_text(encoding="utf-8").splitlines() if line]
    by_phase = {record.get("phase"): record for record in raw}
    if list(by_phase) != PHASES or len(raw) != len(PHASES):
        raise ValueError("provider samples must contain every phase exactly once in order")
    normalized = []
    previous_live = 0
    for phase in PHASES:
        record = by_phase[phase]
        gross = checked_nonnegative(record, "gross_bytes")
        live = checked_nonnegative(record, "live_bytes")
        peak = checked_nonnegative(record, "peak_bytes")
        if peak < live or gross < live:
            raise ValueError(f"invalid gross/live/peak relationship at {phase}")
        folded = []
        attributed = 0
        for item in record.get("stacks", []):
            byte_count = checked_nonnegative(item, "bytes")
            stack = str(item.get("stack", "[unknown]"))
            if not SAFE_STACK.fullmatch(stack):
                stack = "[redacted]"
            if not stack:
                stack = "[unknown]"
            folded.append({"stack": stack, "bytes": byte_count})
            attributed += byte_count
        normalized.append(
            {
                "phase": phase,
                "allocation_count": optional_nonnegative(record, "allocation_count"),
                "allocated_bytes": optional_nonnegative(record, "allocated_bytes", gross),
                "deallocated_bytes": optional_nonnegative(record, "deallocated_bytes"),
                "gross_bytes": gross,
                "live_bytes": live,
                "peak_live_bytes": peak,
                "diff_live_bytes": live - previous_live,
                "folded_stacks": folded,
                "unattributed_bytes": max(0, gross - attributed),
            }
        )
        previous_live = live
    write_json(
        output,
        {
            "schema_version": "hydracache-memory-provider-normalized-v1",
            "provider": provider,
            "phases": normalized,
        },
    )


def checked_nonnegative(value: dict[str, Any], key: str) -> int:
    number = value.get(key)
    if not isinstance(number, int) or number < 0:
        raise ValueError(f"{key} must be a non-negative integer")
    return number


def optional_nonnegative(value: dict[str, Any], key: str, default: int | None = None) -> int | None:
    number = value.get(key, default)
    if number is None:
        return None
    if not isinstance(number, int) or number < 0:
        raise ValueError(f"{key} must be a non-negative integer or null")
    return number


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    commands = result.add_subparsers(dest="command", required=True)
    probe = commands.add_parser("probe")
    probe.add_argument("--output", type=Path, required=True)
    start = commands.add_parser("start")
    start.add_argument("--state", type=Path, required=True)
    start.add_argument("--raw", type=Path, required=True)
    start.add_argument("--pid", type=int, default=os.getpid())
    start.add_argument("--binary", type=Path, required=True)
    start.add_argument("--symbols", type=Path)
    mark = commands.add_parser("mark")
    mark.add_argument("--state", type=Path, required=True)
    mark.add_argument("--phase", required=True)
    snapshot = commands.add_parser("snapshot")
    snapshot.add_argument("--state", type=Path, required=True)
    snapshot.add_argument("--phase", required=True)
    snapshot.add_argument("--metrics", type=Path)
    stop = commands.add_parser("stop")
    stop.add_argument("--state", type=Path, required=True)
    normalize_parser = commands.add_parser("normalize")
    normalize_parser.add_argument("--input", type=Path, required=True)
    normalize_parser.add_argument("--timeline", type=Path, required=True)
    normalize_parser.add_argument("--output", type=Path, required=True)
    return result


def main(provider: str) -> int:
    args = parser().parse_args()
    try:
        if args.command == "probe":
            available, reason = capability(provider)
            write_json(args.output, {"provider": provider, "available": available, "reason": reason})
            return 0 if available else 3
        if args.command == "start":
            available, reason = capability(provider)
            if not available:
                raise RuntimeError(reason)
            if not args.binary.is_file():
                raise ValueError("--binary must identify the exact measured binary")
            symbols = args.symbols or args.binary
            if not symbols.is_file():
                raise ValueError("--symbols must identify the measured symbol file")
            write_json(args.state, {
                "provider": provider,
                "pid": args.pid,
                "raw": str(args.raw),
                "marks": [],
                "stopped": False,
                "binary": str(args.binary.resolve()),
                "binary_digest": file_digest(args.binary),
                "symbols": str(symbols.resolve()),
                "symbols_digest": file_digest(symbols),
                "tool_version": sys.version.split()[0],
                "command": sys.argv,
                "started_monotonic_ns": time.monotonic_ns(),
            })
        elif args.command == "mark":
            validate_phase(args.phase)
            state = read_json(args.state)
            expected = PHASES[len(state["marks"])] if len(state["marks"]) < len(PHASES) else None
            if args.phase != expected:
                raise ValueError(f"expected phase {expected}, got {args.phase}")
            state["marks"].append(args.phase)
            state.setdefault("mark_monotonic_ns", []).append(time.monotonic_ns())
            write_json(args.state, state)
        elif args.command == "snapshot":
            validate_phase(args.phase)
            state = read_json(args.state)
            if not state["marks"] or state["marks"][-1] != args.phase:
                raise ValueError("snapshot phase has no matching latest mark")
            if args.metrics:
                metrics = read_json(args.metrics)
            else:
                rss = resident_bytes(int(state["pid"]))
                metrics = {"gross_bytes": rss, "live_bytes": rss, "peak_bytes": rss, "stacks": []}
            metrics["phase"] = args.phase
            metrics["monotonic_ns"] = time.monotonic_ns()
            append_jsonl(Path(state["raw"]), metrics)
        elif args.command == "stop":
            state = read_json(args.state)
            if state["marks"] != PHASES:
                raise ValueError("stop requires all mandatory phase marks")
            state["stopped"] = True
            write_json(args.state, state)
        elif args.command == "normalize":
            normalize(args.input, args.timeline, args.output, provider)
        return 0
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"{provider} memory provider: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main("system"))
