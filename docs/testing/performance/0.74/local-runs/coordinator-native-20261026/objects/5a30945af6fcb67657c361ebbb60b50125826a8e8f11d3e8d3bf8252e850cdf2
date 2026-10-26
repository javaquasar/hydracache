"""Finite original-wrapper output controls; Python fixtures only, never Cargo."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

from attempt_output_binding import AttemptOutputs
from bound_target_observation import BoundTargetObservation
from dispatch_handoff import DispatchOnce, ExitApi, ObservationCustody
from launch_preflight import pinned
from observation_coordinator import Coordinator
from owned_process_observer import OwnedObserver, Refusal
from target_result_join import ExpectedSource, catalog_for, source_digest


def reserved_source(root):
    # Exact original wrapper inventory, independently collected before spawn.
    names = {s.decode() for s in subprocess.check_output(['git', 'ls-files', '-z', '--',
        'Cargo.toml', 'Cargo.lock', 'deny.toml', '.cargo', 'crates', 'tools'], cwd=root).split(b'\0') if s}
    extra = 'crates/xtask/tests/authority_hex_reader_074.rs'
    if (root / extra).exists():
        names.add(extra)
    source = {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in sorted(names)}
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    return ExpectedSource(head, source_digest(source))


def control(output, mode):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    result = dict(mode=mode, passed=False, first_refusal=None, cleanup_errors=[], cargo_dispatch=False,
                  selected_target_pass=False, full_package_pass=False, admission=False)
    state = dict(owner=None, custody=None, started=None, spawns=0)
    workspace = Path(__file__).resolve().parents[2]
    label = "output-bound-20261025-" + mode
    outputs = leak = session = None
    try:
        (out / "intent.json").write_text(json.dumps(result), encoding="utf-8")
        if mode not in ("healthy", "writer-held"):
            raise Refusal("unknown output binding fixture mode")
        pinned(workspace / "target/run-xtask-package-780-20261024.py", "52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d")
        pinned(workspace / "target/run-source-step-20261024.py", "a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430")
        outputs = AttemptOutputs(label, out, workspace / "target/source-continuation-20261024", reserved_source(workspace))
        api, image = ExitApi(), str(Path(sys.executable).resolve())
        with (out / "wrapper.stdout.log").open("xb") as stdout, (out / "wrapper.stderr.log").open("xb") as stderr:
            def spawn():
                state["started"] = time.monotonic()
                state["spawns"] += 1
                state["owner"] = subprocess.Popen([sys.executable,
                    str(workspace / "target/run-xtask-package-780-20261024.py"), label,
                    sys.executable, "-I", str(Path(__file__).with_name("lineage_control.py")), "node", "2", str(out)],
                    cwd=workspace, stdout=stdout, stderr=stderr)
                return state["owner"]

            def attach(owner):
                root = OwnedObserver(owner, image, api)
                monitor = Coordinator(owner, root, api, [(image, image, image)])
                custody = state["custody"] = ObservationCustody(owner, root, monitor, api)
                while monitor.chain is None and custody.observation is None:
                    custody.tick(time.monotonic() - state["started"])
                    time.sleep(0.05)
                if monitor.chain is None:
                    raise Refusal("synthetic output chain not attached")
                outputs.attach(custody)  # wrapper stream files exist before its child can attach
                return custody

            launch = DispatchOnce(label, out)
            launch.start(spawn, attach, lambda _: None)
            c = state["custody"]
            if launch.first_refusal is not None or c is None:
                raise Refusal("initial output dispatch/attachment refused: " + str(launch.first_refusal))
            session = BoundTargetObservation(launch, outputs, catalog_for(workspace))
            if mode == "writer-held":
                leak = (outputs.path / "stdout.log").open("ab")
            (out / "release").write_bytes(b"release")
            while c.exit_proof is None and c.custody_refusal is None:
                if time.monotonic() - state["started"] >= Coordinator.OUTER_SECONDS:
                    raise Refusal("synthetic output custody outer bound; retain original owner")
                c.tick(time.monotonic() - state["started"])
                time.sleep(0.05)
            try:
                session.finish()
            except Refusal as error:
                result["expected_join_refusal"] = str(error)
                expected = "target command/window/retry drift" if mode == "healthy" else "Win32 error 32"
                if expected not in str(error):
                    raise
            else:
                raise Refusal("synthetic node command must not become a fixed target pass")
            if leak is not None:
                leak.close(); leak = None
                try:
                    session.finish()
                except Refusal as error:
                    if str(error) != "bound result join cannot be repeated":
                        raise
                    result["no_retry_after_writer_close"] = True
                else:
                    raise Refusal("writer failure retried into a pass")
            if not c.release_attempted:
                c.finalize_observation()
            result.update(binding=session.binding, original_owner=outputs.owner_identity,
                          custody=c.result(), real_wrapper_spawns=state["spawns"],
                          output_first_refusal=outputs.first_refusal, passed=True)
    except BaseException as error:
        result["first_refusal"] = f"{type(error).__name__}: {error}"
    finally:
        if leak is not None:
            leak.close()
        owner = state["owner"]
        if owner is not None and owner.poll() is None:
            try:
                if not (out / "release").exists():
                    (out / "release").write_bytes(b"release")
                owner.wait(timeout=810)  # do not kill or reacquire the original wrapper
            except Exception as error:
                result["cleanup_errors"].append(str(error))
        if outputs is not None:
            try:
                outputs.close()
            except Exception as error:
                result["cleanup_errors"].append(str(error))
        result["live_owner_custody_retained"] = owner is not None and owner.poll() is None
        result["passed"] = result["passed"] and not result["cleanup_errors"]
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result["passed"]


def cohort(output):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / "intent.json").write_text(json.dumps(dict(schedule=["healthy", "writer-held"], retry=False, cargo_dispatch=False)), encoding="utf-8")
    results = []
    for mode in ("healthy", "writer-held"):
        passed = control(out / mode, mode)
        results.append(dict(mode=mode, passed=passed))
        if not passed:
            break
    passed = len(results) == 2 and all(row["passed"] for row in results)
    (out / "result.json").write_text(json.dumps(dict(passed=passed, cases=results, admission=False)), encoding="utf-8")
    return passed


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "synthetic-controls":
        raise SystemExit("usage: output_binding_control.py synthetic-controls NEW_OUTPUT_DIR")
    raise SystemExit(0 if cohort(sys.argv[2]) else 1)
