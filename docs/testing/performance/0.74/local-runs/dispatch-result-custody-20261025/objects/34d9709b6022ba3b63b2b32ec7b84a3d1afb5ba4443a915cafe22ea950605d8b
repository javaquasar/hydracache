"""Explicit local prepare/dispatch request. Fixed receipt still denies dispatch."""

import json
from pathlib import Path
import re
import sys

import launch_preflight
from owned_process_observer import Refusal


def workspace():
    return Path(__file__).resolve().parents[2]


def request(mode, label):
    if mode not in ("prepare", "dispatch") or re.fullmatch(r"[a-z][a-z0-9-]{0,63}", label) is None:
        raise Refusal("invalid fixed-target request")
    root = workspace()
    out = root / "target/observer-dispatch-074" / label
    out.mkdir(parents=True, exist_ok=False)
    intent = dict(mode=mode, label=label, receipt_sha256=launch_preflight.RECEIPT_SHA256,
                  retry=False, real_target_spawns=0, original_wrapper_seconds=780)
    with (out / "intent.json").open("x", encoding="utf-8") as output:
        json.dump(intent, output, indent=2)
    try:
        preflight = launch_preflight.evaluate(root)
        if preflight.get("preflight_pass") is not True or preflight.get("dispatch_authorized") is not False:
            raise Refusal("preflight did not confirm the fixed non-dispatching scope")
        result = dict(preflight=preflight, dispatch_attempted=False, real_target_spawns=0,
                      full_package_pass=False, admission=False)
        if mode == "dispatch":
            # A successful preflight is explicitly NOT dispatch authority.
            # There is no process-launch API in this CLI, even under test mocks.
            raise Refusal("fixed launch receipt forbids real-target dispatch")
        result["prepared"] = True
    except Exception as error:
        result = dict(prepared=False, first_refusal=f"{type(error).__name__}: {error}",
                      dispatch_attempted=False, real_target_spawns=0, admission=False)
    with (out / "result.json").open("x", encoding="utf-8") as output:
        json.dump(result, output, indent=2)
    return result


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: dispatch_request.py prepare|dispatch NEW_LABEL")
    result = request(sys.argv[1], sys.argv[2])
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result.get("prepared") else 1)
