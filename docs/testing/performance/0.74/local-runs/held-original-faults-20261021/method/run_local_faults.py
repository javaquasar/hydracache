"""One bounded original held-holder fault session; never retries a refusal."""
import argparse
import json
from pathlib import Path
import signal
import time
import runner as original
import run_preparation_impact as preparation

NAME = "held-original-faults-20261021"
CASES = ("revalidate", "role-before", "role-after", "trust-after")
PRIOR = "directory-preparation-impact-20261021"
PRIOR_SHA = "c1a697c7f1c281d7b0330800d516340ce5cf3c3eee5cd55b26ee637b023b346c"
METHOD = ("runner.py", "run_preparation_impact.py", "run_local_faults.py",
          "review_local_faults.py", "test_local_faults.py", "launch_local_faults.ps1", "attribute_partial.py")


def child_env(repo, case):
    original.require(case in CASES, "Only original remaining fixture cases")
    return {**original.child_env(repo), "HC074_CONSUMING_HELD_LIVE_CASE": case}


def verify_preparation(repo, pins):
    preparation.verify_preparation(repo, pins)
    prior = repo / "docs/testing/performance/0.74/local-runs" / PRIOR
    original.require(original.sha(prior / "inventory.json") == PRIOR_SHA, "Healthy predecessor seal drift")
    members = set()
    for row in original.read_json(prior / "inventory.json")["files"]:
        relative = Path(row["path"])
        original.require(not relative.is_absolute() and ".." not in relative.parts
                         and row["path"] not in members, "Unsafe/duplicate healthy predecessor member")
        members.add(row["path"])
        path = prior / relative
        original.require(not path.is_symlink() and path.resolve().is_relative_to(prior.resolve())
                         and path.stat().st_size == row["bytes"] and original.sha(path) == row["sha256"],
                         "Healthy predecessor member drift")
    original.require({p.relative_to(prior).as_posix() for p in prior.rglob("*") if p.is_file()}
                     == members | {"inventory.json"}, "Healthy predecessor membership drift")


def snapshot(repo, pins):
    result = original.snapshot(repo, pins)
    result["fault_method"] = {n: original.sha(Path(__file__).parent / n) for n in METHOD}
    return result


def run_session(output, pins, observe, execute, clock=time.monotonic, prepare=lambda: None):
    output = Path(output)
    output.mkdir()  # Existing/incomplete packets are never continued.
    started = clock()
    initial, error = None, None
    entries = []
    attempted = 0
    original.write_new(output / "intent.json", dict(schema="original-held-local-faults-074-v1",
        source=pins["source"], schedule=[[lane, case] for lane in original.LANES for case in CASES],
        seed="0x074c051e", child_timeout_seconds=1200, kill_after_seconds=5,
        child_containment_seconds=original.CHILD_SECONDS, session_seconds=original.SESSION_SECONDS,
        max_log_bytes=original.MAX_LOG_BYTES, original_windows_seconds=[300,60,10], **original.SCOPE))
    try:
        prepare()
        initial = observe()
        original.write_new(output / "metadata.json", initial)
        original.validate_snapshot(initial, pins)
        for lane in original.LANES:
            for case in CASES:
                original.require(clock() - started < original.SESSION_SECONDS - original.CHILD_SECONDS - 30,
                                 "Insufficient bounded session budget; no dispatch")
                before = observe()
                original.write_new(output / f"{lane}-{case}-before.json", before)
                original.validate_snapshot(before, pins, initial)
                prefix = output / f"{lane}-{case}"
                original.write_new(prefix.with_suffix(".intent.json"), dict(lane=lane, case=case, **original.SCOPE))
                attempted += 1
                receipt = execute(lane, case, prefix.with_suffix(".log"),
                    min(started + original.SESSION_SECONDS - 30, clock() + original.CHILD_SECONDS))
                entries.append(dict(lane=lane, case=case, **receipt))
                original.write_new(prefix.with_suffix(".exit.json"), entries[-1])
                after = observe()
                original.write_new(output / f"{lane}-{case}-after.json", after)
                original.validate_snapshot(after, pins, initial)
                if receipt["exit"] != 0:
                    break  # Unexpected harness refusal stops this lane; never retry.
    except BaseException as failure:
        error = type(failure).__name__ + ": " + str(failure)
    finally:
        try:
            final = observe()
            original.write_new(output / "postflight.json", final)
            original.validate_snapshot(final, pins, initial)
        except BaseException as failure:
            error = (error or "") + " Postflight: " + type(failure).__name__ + ": " + str(failure)
        result = dict(source=pins["source"], profiles=entries, failure=error,
            measured_profile_invocations=attempted, elapsed_seconds=clock()-started,
            schedule_completed=not error and len(entries)==8 and all(r["exit"]==0 for r in entries),
            full_matrix=False, **original.SCOPE)
        original.write_new(output / "result.json", result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--pins", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    pins = original.read_json(args.pins)
    original.require(args.source == pins["source"], "Source argument drift")
    original.require(args.output.resolve() == repo / "docs/testing/performance/0.74/local-runs" / NAME
                     and args.pins.resolve() == repo / "target" / (NAME + "-pins.json"), "Only this new local packet")
    result = run_session(args.output, pins, lambda: snapshot(repo,args.pins),
        lambda lane,case,log,deadline: original.capture(original.command(lane),child_env(repo,case),log,deadline),
        prepare=lambda: verify_preparation(repo,pins))
    print(json.dumps(result,sort_keys=True),flush=True)
    return 0 if result["schedule_completed"] else 1


if __name__ == "__main__":
    def interrupted(signum,_frame):
        raise InterruptedError(f"Session signal {signum}; no continuation")
    signal.signal(signal.SIGTERM,interrupted)
    signal.signal(signal.SIGHUP,interrupted)
    raise SystemExit(main())
