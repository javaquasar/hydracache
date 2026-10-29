"""Bounded local ordinary-user operation probes; no original admission or start."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("capture", Path(__file__).with_name("runner.py"))
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
PACKET = ROOT / "docs/testing/performance/0.74/local-runs/boot-authority-operations-20261021"
JOBS = (("boot", "boot_operations_selected_cpu_attribution"),
        ("phases", "authority_cpu_actual_phase_attribution"),
        ("leaf", "authority_leaf_cpu_selected_attribution"),
        ("names", "authority_open_name_allocation_cpu_attribution"),
        ("owners", "authority_owners_allocation_attribution"),
        ("full", "validation_leaf_requests_attribution"))
QUALIFICATION = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
CHECKS = ("check","clippy","boot_operations_","authority_","validation_leaf_requests_",
          "boot_","fixed_leaf_path_","worker_trust_input_","kernel_time_","build")


def git(*args):
    return subprocess.run(["/mnt/c/Program Files/Git/cmd/git.exe", "--no-optional-locks", "-C",
        "C:/Workspace/prj/jq/cashe/hydracache-074", *args], capture_output=True, text=True,
        check=True, timeout=10).stdout.strip()


def image(stage, lane):
    receipt = capture.read_json(PACKET / f"{stage}-check-{lane}-exit.json")
    capture.require(not receipt["failure"] and tuple(s["name"] for s in receipt["steps"])==CHECKS
                    and all(s["exit"] == 0 for s in receipt["steps"]), "Preflight failed")
    for item in receipt["inputs"]:
        # Rust tool images do not compile xtask's independent source canaries;
        # those have their own successful, input-bound preflight receipt.
        if item["path"].startswith("crates/xtask/"):
            continue
        capture.require(capture.sha(ROOT/item["path"]) == item["sha256"], "Preflight input drift")
    files = []
    for line in (PACKET / f"{stage}-check-{lane}-build.log").read_text().splitlines():
        if line.startswith("{"):
            row = json.loads(line)
            if row.get("reason") == "compiler-artifact" and row["target"]["name"] == "hydracache_long_run_supervisor_074" and row["profile"]["test"] and row.get("executable"):
                files.append(row["executable"])
    capture.require(len(files) == 1, "Ambiguous image")
    return files[0]


def context(images):
    paths = [Path(__file__), Path(capture.__file__), *map(Path, images.values()),
             ROOT/"docs/testing/performance/0.74/qualification-manifest.toml"]
    return dict(source=git("rev-parse", "HEAD"), dirty=git("status", "--porcelain", "--untracked-files=no"),
        boot=Path("/proc/sys/kernel/random/boot_id").read_text().strip(), uid=os.getuid(), pid=os.getpid(),
        start_ticks=Path("/proc/self/stat").read_text().rsplit(")", 1)[1].split()[19],
        hashes={str(p): capture.sha(p) for p in paths},
        fixtures=sorted(str(p) for pattern in ("hydracache-authority-*","hydracache-open-name-*","hc074-root-fixture-*")
                        for p in Path("/tmp").glob(pattern)),
        config={str(p): capture.sha(p) if p.exists() else None for p in map(Path,
            ("/etc/wsl.conf", "/etc/systemd/timesyncd.conf", "/mnt/c/Users/ArturBuzov/.wslconfig"))})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--stage", required=True, choices=("Baseline", "Candidate"))
    args = parser.parse_args()
    check_stage = ("BaselineCanary" if args.stage=="Baseline" else
                   args.stage+"Repair" if (PACKET/f"{args.stage}Repair-check-xtask-exit.json").exists() else args.stage)
    checked = capture.read_json(PACKET/f"{check_stage}-check-xtask-exit.json")
    capture.require(not checked["failure"] and tuple(r["name"] for r in checked["steps"])==("tests","check","clippy")
                    and all(r["exit"]==0 for r in checked["steps"]), "Source canary preflight failed")
    for item in checked["inputs"]:
        capture.require(capture.sha(ROOT/item["path"])==item["sha256"], "Source canary input drift")
    images = {lane: image(args.stage, lane) for lane in ("194", "188")}
    capture.require(not (PACKET/f"{args.stage}-result.json").exists(), "Never resume")
    capture.write_new(PACKET/f"{args.stage}-intent.json", dict(source=args.source, jobs=JOBS,
        images=images, child_seconds=1200, session_seconds=15000, server=False, product=False))
    initial = None
    failure = None
    results = []
    started = time.monotonic()
    try:
        initial = context(images)
        capture.write_new(PACKET/f"{args.stage}-metadata.json", initial)
        capture.require(initial["source"] == args.source and not initial["dirty"] and initial["uid"] > 0, "Source/user drift")
        capture.require(capture.sha(ROOT/"docs/testing/performance/0.74/qualification-manifest.toml") == QUALIFICATION, "Qualification drift")
        for lane in ("194", "188"):
            for name, selector in JOBS:
                capture.require(time.monotonic()-started < 13700, "Session budget exhausted")
                capture.require(context(images) == initial, "Context drift")
                receipt = capture.capture(["/usr/bin/timeout", "--signal=TERM", "--kill-after=5s", "1200s",
                    images[lane], selector, "--ignored", "--nocapture", "--test-threads=1"],
                    {"PATH": "/usr/bin:/bin"}, PACKET/f"{args.stage}-{lane}-{name}.log",
                    min(started+14900, time.monotonic()+1210))
                receipt.update(stage=args.stage, lane=lane, job=name, source=args.source,
                               image_sha256=capture.sha(images[lane]))
                capture.write_new(PACKET/f"{args.stage}-{lane}-{name}.json", receipt)
                results.append(receipt)
                capture.require(receipt["exit"] == 0 and receipt["containment_failure"] is None, "Child refused; no continuation")
                capture.require(context(images) == initial, "Context drift")
    except BaseException as error:
        failure = type(error).__name__+": "+str(error)
    finally:
        try:
            final = context(images)
            capture.write_new(PACKET/f"{args.stage}-postflight.json", final)
            capture.require(final == initial, "Postflight drift")
        except BaseException as error:
            failure = (failure or "")+" Postflight: "+str(error)
        result = dict(source=args.source, stage=args.stage, results=results, failure=failure,
            complete=not failure and len(results)==12, native_gate_open=False, server=False, product=False)
        capture.write_new(PACKET/f"{args.stage}-result.json", result)
    print(json.dumps(result))
    return 0 if result["complete"] else 1


if __name__ == "__main__":
    def interrupted(number, _frame):
        raise InterruptedError(f"Signal {number}; no continuation")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGHUP, interrupted)
    raise SystemExit(main())
