"""Finite synthetic checks through the exact original 780-second wrapper.

No product/Cargo target is dispatched. The only child command is the previously
checked local Python node fixture. Neither monitor nor cleanup kills the wrapper.
"""

import json
from pathlib import Path
import subprocess
import sys
import time

from launch_preflight import pinned
from observation_coordinator import Coordinator
from owned_process_observer import OwnedObserver, Refusal
from process_lineage import SnapshotApi


class FaultApi(SnapshotApi):
    def snapshot(self):
        raise Refusal("injected census fault")


def control(output, label, mode):
    if mode not in ("healthy", "wrong-path", "census-fault"):
        raise Refusal("unknown synthetic coordinator mode")
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    result = dict(mode=mode, passed=False, first_refusal=None, cleanup_errors=[],
                  cargo_dispatch=False, product_admission=False)
    child = root = monitor = None
    workspace = Path(__file__).resolve().parents[2]
    wrapped = workspace / "target/source-continuation-20261024" / label
    try:
        (out / "intent.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        pinned(workspace / "target/run-xtask-package-780-20261024.py", "52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d")
        pinned(workspace / "target/run-source-step-20261024.py", "a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430")
        if wrapped.exists():
            raise Refusal("original wrapper output already exists")
        api = FaultApi() if mode == "census-fault" else SnapshotApi()
        image = str(Path(sys.executable).resolve())
        argv = [sys.executable, str(workspace / "target/run-xtask-package-780-20261024.py"), label,
                sys.executable, "-I", str(Path(__file__).with_name("lineage_control.py")), "node", "2", str(out)]
        with (out / "wrapper.stdout.log").open("xb") as stdout, (out / "wrapper.stderr.log").open("xb") as stderr:
            started = time.monotonic()  # before Popen; no refreshed observation origin
            child = subprocess.Popen(argv, cwd=workspace, stdout=stdout, stderr=stderr)
            root = OwnedObserver(child, image, api)
            policy = [image] * 3
            if mode == "wrong-path":
                policy[-1] = str(Path(image).parent / "foreign" / Path(image).name)
            monitor = Coordinator(child, root, api, [tuple(policy)])
            while not monitor.owner_exit_observed and not monitor.outer_expired:
                monitor.tick(time.monotonic() - started)
                if monitor.chain is not None or monitor.first_refusal is not None:
                    if not (out / "release").exists():
                        (out / "release").write_bytes(b"release")
                if not monitor.owner_exit_observed:
                    time.sleep(0.05)
            result["observation"] = monitor.result()
        expected = {"healthy": None, "wrong-path": "held lineage image/PID mismatch",
                    "census-fault": "injected census fault"}[mode]
        if monitor.first_refusal != expected or not monitor.owner_exit_observed or monitor.owner_returncode != 0:
            raise Refusal("unexpected coordinator outcome")
        if (mode == "healthy") != monitor.result()["observations_complete"]:
            raise Refusal("observation completion mismatch")
        before = json.loads((wrapped / "intent.json").read_bytes())
        after = json.loads((wrapped / "result.json").read_bytes())
        if before["bound_seconds"] != 780 or after["exit"] != 0 or after["outer_refusal"] is not None or not after["source_unchanged"] or not after["head_unchanged"]:
            raise Refusal("original wrapper failed or changed")
        if (wrapped / "owned-cleanup.log").exists():
            raise Refusal("original wrapper performed unexpected early cleanup")
        if sorted((wrapped / "stdout.log").read_text().splitlines()) != ["node-0-complete", "node-1-complete", "node-2-complete"] or (wrapped / "stderr.log").read_bytes():
            raise Refusal("original wrapped fixture did not finish normally")
        result["passed"] = True
    except BaseException as error:
        result["first_refusal"] = f"{type(error).__name__}: {error}"
    finally:
        if child is not None and child.poll() is None:
            # The synthetic nodes are bounded and release cooperatively. Drain
            # under the unchanged owner's bound, never kill it to fix telemetry.
            try:
                if not (out / "release").exists():
                    (out / "release").write_bytes(b"release")
                child.wait(timeout=810)
            except Exception as error:
                result["cleanup_errors"].append(str(error))
                result["passed"] = False
        if monitor is not None and child.poll() is not None:
            try:
                if not monitor.owner_exit_observed and not monitor.outer_expired:
                    monitor.tick(time.monotonic() - started)
                monitor.close()
            except Exception as error:
                result["cleanup_errors"].append(str(error))
                result["passed"] = False
        if root is not None and child.poll() is not None:
            try:
                root.close()
            except Exception as error:
                result["cleanup_errors"].append(str(error))
                result["passed"] = False
        result["live_owner_custody_retained"] = child is not None and child.poll() is None
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result["passed"]


def cohort(output):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    schedule = ("healthy", "wrong-path", "census-fault")
    (out / "intent.json").write_text(json.dumps(dict(schedule=schedule, retry=False, cargo_dispatch=False)), encoding="utf-8")
    results = []
    for mode in schedule:
        passed = control(out / mode, "coordinator-native-20261025-" + mode, mode)
        results.append(dict(mode=mode, passed=passed))
        if not passed:
            break
    passed = len(results) == 3 and all(row["passed"] for row in results)
    (out / "result.json").write_text(json.dumps(dict(passed=passed, results=results, admission=False), indent=2), encoding="utf-8")
    return passed


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "synthetic-controls":
        raise SystemExit("usage: coordinator_control.py synthetic-controls NEW_OUTPUT_DIR")
    raise SystemExit(0 if cohort(sys.argv[2]) else 1)
