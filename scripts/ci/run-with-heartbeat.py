#!/usr/bin/env python3
"""Run a CI command with monotonic heartbeats, a hard deadline, and a receipt."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--timeout-seconds", required=True, type=float)
    parser.add_argument("--heartbeat-seconds", type=float, default=30.0)
    parser.add_argument("--status-json", required=True, type=Path)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--attempt-id", default=os.environ.get("GITHUB_RUN_ATTEMPT", "local"))
    parser.add_argument("--cwd", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ["--"]:
        args.command = args.command[1:]
    if not args.command:
        parser.error("a command is required after --")
    if args.timeout_seconds <= 0 or args.heartbeat_seconds <= 0:
        parser.error("timeouts must be positive")
    return args


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def write_receipt(path: Path, receipt: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        json.dump(receipt, handle, indent=2, sort_keys=True)
        handle.write("\n")
        temporary = Path(handle.name)
    os.replace(temporary, path)


def terminate_tree(process: subprocess.Popen[object]) -> None:
    if process.poll() is not None:
        return
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    else:
        try:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
                return
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def main() -> int:
    args = parse_args()
    started_at = utc_now()
    started = time.monotonic()
    classification = "runner-loss"
    exit_code: int | None = None
    process: subprocess.Popen[object] | None = None
    log_handle = None
    detail = "watchdog did not reach a terminal state"

    try:
        creation_flags = subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0
        if args.log_file is not None:
            args.log_file.parent.mkdir(parents=True, exist_ok=True)
            log_handle = args.log_file.open("wb")
        process = subprocess.Popen(
            args.command,
            cwd=args.cwd,
            stdout=log_handle,
            stderr=subprocess.STDOUT if log_handle is not None else None,
            start_new_session=os.name != "nt",
            creationflags=creation_flags,
        )
        deadline = started + args.timeout_seconds
        next_heartbeat = started
        while True:
            exit_code = process.poll()
            now = time.monotonic()
            if exit_code is not None:
                classification = "success" if exit_code == 0 else "product-failure"
                detail = f"child exited with code {exit_code}"
                break
            if now >= deadline:
                classification = "timeout"
                detail = f"hard deadline of {args.timeout_seconds:g}s exceeded"
                terminate_tree(process)
                exit_code = process.poll()
                break
            if now >= next_heartbeat:
                elapsed = now - started
                remaining = max(0.0, deadline - now)
                print(
                    f"watchdog heartbeat attempt={args.attempt_id} "
                    f"elapsed={elapsed:.1f}s remaining={remaining:.1f}s pid={process.pid}",
                    flush=True,
                )
                next_heartbeat = now + args.heartbeat_seconds
            time.sleep(min(0.2, max(0.01, deadline - now)))
    except FileNotFoundError as error:
        classification = "tool-unavailable"
        detail = str(error)
        exit_code = 127
    except KeyboardInterrupt:
        classification = "cancelled"
        detail = "watchdog interrupted"
        if process is not None:
            terminate_tree(process)
        exit_code = 130
    except Exception as error:  # noqa: BLE001 - runner failures must become receipts
        classification = "runner-loss"
        detail = f"{type(error).__name__}: {error}"
        if process is not None:
            terminate_tree(process)
        exit_code = 125
    finally:
        if log_handle is not None:
            log_handle.close()
        elapsed = time.monotonic() - started
        receipt = {
            "schema_version": 1,
            "attempt_id": args.attempt_id,
            "classification": classification,
            "command": args.command,
            "detail": detail,
            "elapsed_seconds": round(elapsed, 3),
            "exit_code": exit_code,
            "finished_at": utc_now(),
            "log_file": str(args.log_file) if args.log_file is not None else None,
            "started_at": started_at,
        }
        write_receipt(args.status_json, receipt)
        print(
            f"watchdog result classification={classification} "
            f"elapsed={elapsed:.1f}s receipt={args.status_json}",
            flush=True,
        )

    if classification == "success":
        return 0
    if classification == "timeout":
        return 124
    return exit_code if isinstance(exit_code, int) and exit_code != 0 else 125


if __name__ == "__main__":
    raise SystemExit(main())
