"""One original-owner projection/materialization call per compiler. No retries."""
import argparse
import json
from pathlib import Path
import re
import signal
import time
import runner as base

NAME = "held-material-live-20261021"
METHOD = ("runner.py", "run_material_live.py", "launch_material_live.ps1", "test_material_live.py",
          "review_material_live.py", "attribute_partial.py")


def child_env(repo):
    return {**base.child_env(repo), "HC074_HELD_MATERIAL_LIVE": "original-once-v1"}


def verify_preparation(repo, pins):
    base.require(re.fullmatch(r"[0-9a-f]{40}", pins["source"]), "Source shape")
    changed = base.git(repo, "diff", "--name-only", "215626cb", pins["source"], "--", "tools/long-run-supervisor-074").splitlines()
    base.require(set(changed) == {
        "tools/long-run-supervisor-074/src/diagnostic_held_unit_material.rs",
        "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs",
        "tools/long-run-supervisor-074/tests/support/held_material_live_api.rs"}, "Only exact test adapter changes allowed")
    base.git(repo, "diff", "--exit-code", "215626cb", pins["source"], "--", "Cargo.lock", "docs/COMPAT.md", "docs/testing/gated-test-registry.toml", "docs/testing/performance/0.74/qualification-manifest.toml")
    prior = repo / "docs/testing/performance/0.74/local-runs" / base.PRIOR
    base.require(base.sha(prior / "inventory.json") == base.PRIOR_HASH, "Bundle receipt seal drift")
    base.require(pins["bundle"] == base.read_json(prior / "context-Verified-Before.json")["bundle"], "Original bundle drift")
    fresh = base.read_json(pins["preparation"])
    base.require(set(fresh) == set(base.LANES) == set(pins["images"]), "Lane set")
    for lane in base.LANES:
        row = fresh[lane]
        base.require(row["source"] == pins["source"] and type(row["exit"]) is int and row["exit"] == 0
                     and row["image"] == base.IMAGES[lane] and row["sha256"] == pins["images"][lane], "Fresh image binding")
        log = Path(pins["preparation"]).parent / row["build_log"]
        base.require(log.parent == Path(pins["preparation"]).parent and base.sha(log) == row["build_log_sha256"], "Build log binding")
        artifacts = [json.loads(line) for line in log.read_text().splitlines() if line.startswith("{")]
        images = [a for a in artifacts if a.get("reason") == "compiler-artifact"
                  and a["target"]["name"] == "hydracache_long_run_supervisor_074" and a["profile"]["test"] and a.get("executable")]
        base.require(len(images) == 1 and images[0]["features"] == [] and images[0]["executable"] == row["image"], "Default feature artifact")


def snapshot(repo, pins):
    value = base.snapshot(repo, pins)
    value["material_method"] = {name: base.sha(Path(__file__).parent / name) for name in METHOD}
    return value


def run_session(output, pins, observe, execute, clock=time.monotonic, prepare=lambda: None):
    output = Path(output)
    output.mkdir()
    started = clock()
    initial, error, entries = None, None, []
    base.write_new(output / "intent.json", dict(schema="original-held-material-live-074-v1", source=pins["source"],
        schedule=list(base.LANES), calls_per_lane=1, seed="0x074c051e", original_windows_seconds=[300,60,10],
        child_timeout_seconds=1200, kill_after_seconds=5, child_containment_seconds=base.CHILD_SECONDS,
        session_seconds=base.SESSION_SECONDS, max_log_bytes=base.MAX_LOG_BYTES, **base.SCOPE))
    try:
        prepare()
        initial = observe()
        base.write_new(output / "metadata.json", initial)
        base.validate_snapshot(initial, pins)
        for lane in base.LANES:
            base.require(clock() - started < base.SESSION_SECONDS - base.CHILD_SECONDS - 30, "No remaining containment budget")
            before = observe()
            base.write_new(output / f"{lane}-before.json", before)
            base.validate_snapshot(before, pins, initial)
            base.write_new(output / f"{lane}.intent.json", dict(lane=lane, case="healthy", material_mode="original-once-v1", **base.SCOPE))
            receipt = execute(lane, output / f"{lane}.log", min(started + base.SESSION_SECONDS - 30, clock() + base.CHILD_SECONDS))
            entries.append(dict(lane=lane, **receipt))
            base.write_new(output / f"{lane}.exit.json", entries[-1])
            after = observe()
            base.write_new(output / f"{lane}-after.json", after)
            base.validate_snapshot(after, pins, initial)
            # The other compiler is a preregistered independent lane, not a retry.
    except BaseException as failure:
        error = type(failure).__name__ + ": " + str(failure)
    finally:
        try:
            final = observe()
            base.write_new(output / "postflight.json", final)
            base.validate_snapshot(final, pins, initial)
        except BaseException as failure:
            error = (error or "") + " Postflight: " + type(failure).__name__ + ": " + str(failure)
        result = dict(source=pins["source"], profiles=entries, failure=error, elapsed_seconds=clock()-started,
            schedule_completed=not error and len(entries)==2 and all(type(r["exit"]) is int and r["exit"]==0 for r in entries),
            full_matrix=False, **base.SCOPE)
        base.write_new(output / "result.json", result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--pins", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    pins = base.read_json(args.pins)
    base.require(args.source == pins["source"] and args.output.resolve() == repo / "docs/testing/performance/0.74/local-runs" / NAME
                 and args.pins.resolve() == repo / "target" / (NAME+"-pins.json"), "Only this new packet at exact source")
    result = run_session(args.output, pins, lambda: snapshot(repo,args.pins),
        lambda lane,log,deadline: base.capture(base.command(lane), child_env(repo), log, deadline),
        prepare=lambda: verify_preparation(repo,pins))
    print(json.dumps(result,sort_keys=True),flush=True)
    return 0 if result["schedule_completed"] else 1


if __name__ == "__main__":
    def interrupted(signum, _frame):
        raise InterruptedError(f"Session signal {signum}; no continuation")
    signal.signal(signal.SIGTERM,interrupted)
    signal.signal(signal.SIGHUP,interrupted)
    raise SystemExit(main())
