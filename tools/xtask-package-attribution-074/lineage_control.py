"""Finite synthetic Windows lineage controls. No Cargo or server entry point."""

import json
import ntpath
from pathlib import Path
import subprocess
import sys
import time

def node(depth, output):
    """Each synthetic node owns/reaps its direct child and has a local deadline."""
    if depth not in (0, 1, 2, 3):
        raise RuntimeError("synthetic node depth outside 0..3")
    out, child = Path(output), None
    deadline = time.monotonic() + 8
    try:
        if depth:
            child = subprocess.Popen([sys.executable, "-I", str(Path(__file__).resolve()),
                                      "node", str(depth - 1), str(out)])
        else:
            (out / "leaf-ready").write_bytes(b"ready")
        while not (out / "release").exists():
            if time.monotonic() >= deadline:
                raise RuntimeError("synthetic node deadline")
            time.sleep(0.01)
        if child is not None and child.wait(timeout=3) != 0:
            raise RuntimeError("synthetic descendant refused")
        print(f"node-{depth}-complete", flush=True)
    finally:
        if child is not None and child.poll() is None:
            child.kill()
            child.wait(timeout=3)


def control(output, depth, wrong_path=False):
    from owned_process_observer import OwnedObserver, Refusal
    from process_lineage import HeldLineage, SnapshotApi

    if depth not in (2, 3):
        raise Refusal("control depth must be 2 or 3")
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    result = dict(schema=1, scope="synthetic-lineage-only", depth=depth,
                  expected_refusal="held lineage image/PID mismatch" if wrong_path else None,
                  passed=False, refusal=None, cleanup_errors=[], identities=[], samples=[],
                  cargo_dispatch=False, product_admission=False)
    child = root = lineage = None
    try:
        (out / "intent.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        api = SnapshotApi()
        image = str(Path(sys.executable).resolve())
        with (out / "stdout.log").open("xb") as stdout, (out / "stderr.log").open("xb") as stderr:
            child = subprocess.Popen([sys.executable, "-I", str(Path(__file__).resolve()),
                                      "node", str(depth), str(out)],
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
            root = OwnedObserver(child, image, api)
            deadline = time.monotonic() + 5
            while not (out / "leaf-ready").exists():
                if child.poll() is not None:
                    raise Refusal("synthetic root exited before readiness")
                if time.monotonic() >= deadline:
                    raise Refusal("synthetic readiness deadline")
                time.sleep(0.01)
            policy = [image] * depth
            if wrong_path:
                policy[-1] = ntpath.join(r"C:\not-admitted-lineage-fixture", ntpath.basename(image))
            try:
                lineage = HeldLineage(root, api, [tuple(policy)])
            except Refusal as error:
                if not wrong_path or str(error) != result["expected_refusal"]:
                    raise
                result["refusal"] = str(error)
            else:
                if wrong_path:
                    raise Refusal("negative control unexpectedly admitted")
                result["identities"] = lineage.identities
            (out / "release").write_bytes(b"release")
            result["exit_code"] = child.wait(timeout=10)
            if result["exit_code"] != 0:
                raise Refusal("synthetic root failed after release")
            if lineage is not None:
                if not lineage.sample()["terminal"]:
                    raise Refusal("leaf not terminal after root reaped descendants")
                result["samples"] = lineage.samples
            root.sample()
            result["root_samples"] = root.samples
        lines = (out / "stdout.log").read_text().splitlines()
        if sorted(lines) != sorted(f"node-{i}-complete" for i in range(depth + 1)) or (out / "stderr.log").read_bytes():
            raise Refusal("synthetic node streams mismatch")
        result["passed"] = True
    except BaseException as error:
        if result["refusal"] is None:
            result["refusal"] = f"{type(error).__name__}: {error}"
        else:
            result["later_refusal"] = f"{type(error).__name__}: {error}"
    finally:
        if child is not None and child.poll() is None:
            try:
                if not (out / "release").exists():
                    (out / "release").write_bytes(b"release")
                child.wait(timeout=12)
            except Exception as error:
                result["passed"] = False
                result["cleanup_errors"].append(str(error))
                try:
                    child.kill()
                    child.wait(timeout=3)
                except Exception as kill_error:
                    result["cleanup_errors"].append(str(kill_error))
        for observed in (lineage, root):
            if observed is not None:
                try:
                    observed.close()
                except Exception as error:
                    result["passed"] = False
                    result["cleanup_errors"].append(str(error))
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result["passed"]


def cohort(output):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    schedule = (("two-level", 2, False), ("three-level", 3, False), ("wrong-path", 2, True))
    (out / "intent.json").write_text(json.dumps(dict(schedule=schedule, retry=False,
                                                    cargo_dispatch=False), indent=2), encoding="utf-8")
    results = []
    for label, depth, wrong_path in schedule:
        passed = control(out / label, depth, wrong_path)
        results.append(dict(label=label, passed=passed))
        if not passed:
            break
    passed = len(results) == 3 and all(row["passed"] for row in results)
    (out / "result.json").write_text(json.dumps(dict(passed=passed, results=results,
                                                    admission=False), indent=2), encoding="utf-8")
    return passed


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "node":
        node(int(sys.argv[2]), sys.argv[3])
    elif len(sys.argv) == 3 and sys.argv[1] == "synthetic-controls":
        raise SystemExit(0 if cohort(sys.argv[2]) else 1)
    else:
        raise SystemExit("usage: lineage_control.py synthetic-controls NEW_OUTPUT_DIR")
