"""One new full local preparation session at the bounded directory candidate.

Reuse the frozen original runner's schedule, environment and containment without
modifying it. Only preparation verification accepts the already-tested candidate.
"""
import argparse
import json
import re
import signal
from pathlib import Path
import runner as original

NAME = "directory-preparation-impact-20261021"
CANDIDATE = "3d3a2180c6b54ea9156ae2bac609f36966359908"
PRIOR = "boot-authority-operations-20261021"
PRIOR_SHA = "c5079760b9eeda76904b9addad40a5d3593b5535e447d320c790ff6392c26bcb"
METHOD = ("runner.py", "run_preparation_impact.py", "launch_preparation_impact.ps1",
          "review_preparation_impact.py", "test_preparation_impact.py")


def verify_preparation(repo, pins):
    original.require(re.fullmatch(r"[0-9a-f]{40}", pins["source"]), "Invalid source")
    original.git(repo, "diff", "--exit-code", CANDIDATE, pins["source"], "--",
                 "tools/long-run-supervisor-074", "crates", "Cargo.lock")
    prior = repo / "docs/testing/performance/0.74/local-runs" / PRIOR
    original.require(original.sha(prior / "inventory.json") == PRIOR_SHA, "Prior seal drift")
    members = set()
    for row in original.read_json(prior / "inventory.json")["files"]:
        relative = Path(row["path"])
        original.require(not relative.is_absolute() and ".." not in relative.parts
                         and row["path"] not in members, "Unsafe/duplicate prior member")
        members.add(row["path"])
        path = prior / relative
        original.require(not path.is_symlink() and path.resolve().is_relative_to(prior.resolve())
                         and path.stat().st_size == row["bytes"]
                         and original.sha(path) == row["sha256"], "Prior member drift")
    original.require({p.relative_to(prior).as_posix() for p in prior.rglob("*") if p.is_file()}
                     == members | {"inventory.json"}, "Prior membership drift")
    baseline = repo / "docs/testing/performance/0.74/local-runs" / original.PRIOR
    original.require(original.sha(baseline / "inventory.json") == original.PRIOR_HASH,
                     "Original bundle receipt seal drift")
    original.require(pins["bundle"] == original.read_json(baseline / "context-Verified-Before.json")["bundle"],
                     "Original nine-file bundle changed")
    original.require(set(pins["images"]) == set(original.LANES), "Lane set changed")
    fresh = original.read_json(pins["preparation"])
    original.require(set(fresh) == set(original.LANES), "Fresh lane set changed")
    for lane in original.LANES:
        row = fresh[lane]
        original.require(row["source"] == pins["source"] and type(row["exit"]) is int and row["exit"] == 0
                         and row["image"] == original.IMAGES[lane]
                         and row["sha256"] == pins["images"][lane], "Fresh image binding drift")
        log = Path(pins["preparation"]).parent / row["build_log"]
        original.require(log.parent == Path(pins["preparation"]).parent
                         and original.sha(log) == row["build_log_sha256"], "Build log drift")
        artifacts = [json.loads(line) for line in log.read_text().splitlines() if line.startswith("{")]
        images = [a for a in artifacts if a.get("reason") == "compiler-artifact"
                  and a["target"]["name"] == "hydracache_long_run_supervisor_074"
                  and a["profile"]["test"] and a.get("executable")]
        original.require(len(images) == 1 and images[0]["features"] == []
                         and images[0]["executable"] == original.IMAGES[lane], "Not a fresh default-feature image")


def snapshot(repo, pins):
    value = original.snapshot(repo, pins)
    value["impact_method"] = {name: original.sha(Path(__file__).parent / name) for name in METHOD}
    return value


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
                     and args.pins.resolve() == repo / "target" / (NAME + "-pins.json"), "Only this new packet")
    result = original.run_session(args.output, pins, lambda: snapshot(repo, args.pins),
        lambda lane, log, deadline: original.capture(original.command(lane), original.child_env(repo), log, deadline),
        prepare=lambda: verify_preparation(repo, pins))
    print(json.dumps(result, sort_keys=True), flush=True)
    return 0 if result["schedule_completed"] else 1


if __name__ == "__main__":
    def interrupted(signum, _frame):
        raise InterruptedError(f"Session signal {signum}; no continuation")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGHUP, interrupted)
    raise SystemExit(main())
