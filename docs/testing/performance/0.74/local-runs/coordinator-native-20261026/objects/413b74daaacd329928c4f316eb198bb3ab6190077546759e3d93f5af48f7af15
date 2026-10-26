"""Two finite synthetic dispatch/custody controls; never dispatches Cargo."""

import json
from pathlib import Path
import subprocess
import sys
import time

from dispatch_handoff import DispatchOnce, ExitApi, ObservationCustody
from launch_preflight import pinned
from observation_coordinator import Coordinator
from owned_process_observer import OwnedObserver, Refusal


class CountingApi(ExitApi):
    def __init__(self):
        super().__init__()
        self.counter_reads = self.census_reads = 0

    def read(self, handle):
        self.counter_reads += 1
        return super().read(handle)

    def snapshot(self):
        self.census_reads += 1
        return super().snapshot()


def control(output, mode):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    result = dict(mode=mode, passed=False, first_refusal=None, cleanup_errors=[], cargo_dispatch=False, admission=False)
    context = dict(owner=None, custody=None, started=None, spawns=0)
    workspace = Path(__file__).resolve().parents[2]
    label = "handoff-native-20261025-" + mode
    try:
        (out / "intent.json").write_text(json.dumps(result), encoding="utf-8")
        if mode not in ("healthy", "lost-ack"):
            raise Refusal("unknown synthetic handoff mode")
        pinned(workspace / "target/run-xtask-package-780-20261024.py", "52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d")
        pinned(workspace / "target/run-source-step-20261024.py", "a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430")
        if (workspace / "target/source-continuation-20261024" / label).exists():
            raise Refusal("original wrapper output already exists")
        api = CountingApi()
        image = str(Path(sys.executable).resolve())
        with (out / "wrapper.stdout.log").open("xb") as stdout, (out / "wrapper.stderr.log").open("xb") as stderr:
            def spawn():
                context["spawns"] += 1
                context["started"] = time.monotonic()
                context["owner"] = subprocess.Popen([sys.executable,
                    str(workspace / "target/run-xtask-package-780-20261024.py"), label,
                    sys.executable, "-I", str(Path(__file__).with_name("lineage_control.py")), "node", "2", str(out)],
                    cwd=workspace, stdout=stdout, stderr=stderr)
                return context["owner"]

            def attach(owner):
                root = OwnedObserver(owner, image, api)
                monitor = Coordinator(owner, root, api, [(image, image, image)])
                custody = ObservationCustody(owner, root, monitor, api)
                context["custody"] = custody
                while monitor.chain is None and custody.observation is None:
                    custody.tick(time.monotonic() - context["started"])
                    time.sleep(0.05)
                if monitor.chain is None:
                    raise Refusal("synthetic chain did not attach")
                return custody

            def ack(attempt_id):
                if mode == "lost-ack":
                    raise OSError("injected lost spawn ACK")
                (out / "ack.json").write_text(json.dumps(dict(attempt_id=attempt_id)), encoding="utf-8")

            launch = DispatchOnce("synthetic-" + mode, out)
            result["dispatch"] = launch.start(spawn, attach, ack)
            custody = context["custody"]
            if custody is None or launch.custody is not custody:
                raise Refusal("initial synthetic attachment failed")
            if mode == "lost-ack":
                before_reads = (api.counter_reads, api.census_reads)
                if context["owner"].poll() is not None:
                    raise Refusal("owner already exited before lost-ACK replay control")
                replay = DispatchOnce("synthetic-" + mode, out)
                result["restart_request"] = replay.start(spawn, attach, ack)
                if replay.owner is not None or context["spawns"] != 1:
                    raise Refusal("lost ACK caused a second spawn")
            (out / "release").write_bytes(b"release")
            while custody.exit_proof is None and custody.custody_refusal is None:
                custody.tick(time.monotonic() - context["started"])
                time.sleep(0.05)
            custody.finalize_observation()
            result["custody"] = custody.result()
            if mode == "lost-ack":
                result["post_freeze_counter_reads"] = api.counter_reads - before_reads[0]
                result["post_freeze_census_reads"] = api.census_reads - before_reads[1]
                if result["post_freeze_counter_reads"] or result["post_freeze_census_reads"]:
                    raise Refusal("profiling resumed after lost ACK")
            expected = None if mode == "healthy" else "dispatch acknowledgement refused: injected lost spawn ACK"
            if custody.first_refusal != expected or custody.exit_proof["owner_returncode"] != 0 or len(custody.exit_proof["witnesses"]) != 4:
                raise Refusal("unexpected held-exit handoff")
            attached = json.loads((out / "owner-attached.json").read_bytes())
            witness = custody.exit_proof["witnesses"][0]
            if (attached["pid"], attached["creation_100ns"]) != (witness["pid"], witness["creation_100ns"]):
                raise Refusal("original owner changed across ACK/handoff")
            result["real_wrapper_spawns"] = context["spawns"]
        wrapped = workspace / "target/source-continuation-20261024" / label
        receipt = json.loads((wrapped / "result.json").read_bytes())
        if receipt["exit"] != 0 or receipt["outer_refusal"] is not None or not receipt["source_unchanged"] or not receipt["head_unchanged"]:
            raise Refusal("original synthetic wrapper did not finish unchanged")
        if sorted((wrapped / "stdout.log").read_text().splitlines()) != ["node-0-complete", "node-1-complete", "node-2-complete"]:
            raise Refusal("synthetic descendants did not complete")
        result["passed"] = True
    except BaseException as error:
        result["first_refusal"] = f"{type(error).__name__}: {error}"
    finally:
        owner = context["owner"]
        if owner is not None and owner.poll() is None:
            try:
                if not (out / "release").exists():
                    (out / "release").write_bytes(b"release")
                owner.wait(timeout=810)  # drain only; never kill the original wrapper
            except Exception as error:
                result["cleanup_errors"].append(str(error))
        result["live_owner_custody_retained"] = owner is not None and owner.poll() is None
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result["passed"]


def cohort(output):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / "intent.json").write_text(json.dumps(dict(schedule=["healthy", "lost-ack"], retry=False, cargo_dispatch=False)), encoding="utf-8")
    results = []
    for mode in ("healthy", "lost-ack"):
        passed = control(out / mode, mode)
        results.append(dict(mode=mode, passed=passed))
        if not passed:
            break
    passed = len(results) == 2 and all(row["passed"] for row in results)
    (out / "result.json").write_text(json.dumps(dict(passed=passed, results=results, admission=False), indent=2), encoding="utf-8")
    return passed


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "synthetic-controls":
        raise SystemExit("usage: handoff_control.py synthetic-controls NEW_OUTPUT_DIR")
    raise SystemExit(0 if cohort(sys.argv[2]) else 1)
