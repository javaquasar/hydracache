"""Private, local-only healthy preparation profiler; not an execution authority.

Run through launch.ps1 after freezing source. Never resumes an existing packet.
All test admission checks remain in the unchanged compiled Rust harness.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import subprocess
import time


BASELINE = "b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d"
PRIOR = "validation-round-residual-20261021"
PRIOR_HASH = "15c86f32b3f56c0c86ac526ad72b54b32aa3cf75625543cea3dd3d9aee9c2807"
QUALIFICATION = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
LANES = ("194", "188")
IMAGES = {
    "194": "/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098",
    "188": "/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234",
}
CHILD_SECONDS = 1210  # Outer containment; inner timeout remains 1200 + 5 seconds.
SESSION_SECONDS = 7500
MAX_LOG_BYTES = 32 * 1024 * 1024
SCOPE = dict(server=False, product=False, original_send=False, native_gate_open=False)


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write_new(path, value):
    # O_EXCL is intentional: an incomplete or refused packet cannot be retried.
    with Path(path).open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def git(repo, *args):
    result = subprocess.run(
        ["/usr/bin/git", "-c", f"safe.directory={repo}", "-C", str(repo), *args], capture_output=True,
        timeout=10, check=True, text=True,
    )
    return result.stdout.strip()


def fixtures():
    # Only observations of the harness's bounded names, never a cleanup target.
    return sorted(str(p) for root in ("/tmp", "/sys/fs/cgroup")
                  for p in Path(root).glob("hc074-root-fixture-*"))


def bundle_inventory(bundle):
    # Preserve the existing Windows receipt's case-insensitive name ordering.
    return [dict(name=p.name, bytes=p.stat().st_size, sha256=sha(p))
            for p in sorted(Path(bundle).iterdir(), key=lambda p: p.name.casefold())]


def snapshot(repo, pins):
    bundle = repo / "target/diagnostic-start-material-bundle-20261017"
    config = [Path("/etc/wsl.conf"), Path("/etc/systemd/timesyncd.conf"),
              Path("/mnt/c/Users/ArturBuzov/.wslconfig")]
    return dict(
        source=git(repo, "rev-parse", "HEAD"),
        tracked_dirty=git(repo, "status", "--porcelain", "--untracked-files=no"),
        boot=Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
        kernel=os.uname().release, uid=os.getuid(), pid=os.getpid(),
        start_ticks=Path("/proc/self/stat").read_text().rsplit(")", 1)[1].split()[19],
        config={str(p): sha(p) if p.exists() else None for p in config},
        fixtures=fixtures(),
        bundle=bundle_inventory(bundle),
        images={lane: sha(path) for lane, path in IMAGES.items()},
        qualification_sha256=sha(repo / "docs/testing/performance/0.74/qualification-manifest.toml"),
        driver_sha256=sha(Path(__file__)), pins_sha256=sha(pins),
        preparation_sha256=sha(read_json(pins)["preparation"]),
    )


def validate_snapshot(current, pins, initial=None):
    require(current["source"] == pins["source"] and not current["tracked_dirty"], "Source drift")
    require(current["uid"] == 0 and not current["fixtures"], "Root context or fixture residue")
    require(re.fullmatch(r"[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}", current["boot"]), "Invalid boot")
    require(current["bundle"] == pins["bundle"], "Original bundle changed")
    require(current["images"] == pins["images"], "Pinned image changed")
    require(current["qualification_sha256"] == QUALIFICATION, "Qualification changed")
    if initial is not None:
        require(current == initial, "Session metadata changed (boot/config/source/image/process)")


def command(lane):
    return ["/usr/bin/timeout", "--signal=TERM", "--kill-after=5s", "1200s",
            IMAGES[lane], "privileged_consuming_held_live_matrix", "--ignored",
            "--nocapture", "--test-threads=1"]


def child_env(repo):
    # Do not forward filters, allocator knobs or a changed synthetic fixture case.
    return dict(PATH="/usr/bin:/bin", HOME="/root", RUSTUP_AUTO_INSTALL="0",
                HC074_OWNED_ROOT_FIXTURE="local-only-v1",
                HC074_START_MATERIAL_BUNDLE=str(repo / "target/diagnostic-start-material-bundle-20261017"),
                HC074_CONSUMING_HELD_LIVE_CASE="healthy", HC074_CONSUMING_HELD_COST="1")


def stop_group(process):
    # Only the fresh child session we created; never host cgroup or other PIDs.
    for sig, wait in ((signal.SIGTERM, 5), (signal.SIGKILL, 1)):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        try:
            process.wait(timeout=wait)
        except subprocess.TimeoutExpired:
            continue
        # Kill residual pipe holders in our group even if its leader exited.
        if sig == signal.SIGTERM:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        break
    process.wait(timeout=1)


def capture(argv, env, log, deadline, max_bytes=MAX_LOG_BYTES):
    started = time.monotonic()
    count = 0
    failure = None
    with Path(log).open("xb") as stream:
        process = subprocess.Popen(argv, env=env, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(process.stdout, selectors.EVENT_READ)
                while selector.get_map():
                    require(time.monotonic() < deadline, "Child/session containment timeout")
                    for key, _ in selector.select(timeout=0.2):
                        data = os.read(key.fd, 65536)
                        if not data:
                            selector.unregister(key.fileobj)
                            continue
                        require(count + len(data) <= max_bytes, "Profile log limit")
                        stream.write(data)
                        count += len(data)
                process.wait(timeout=max(0.001, deadline - time.monotonic()))
        except BaseException as error:
            failure = type(error).__name__ + ": " + str(error)
            raise
        finally:
            # EOF/leader exit alone is not a claim that every writer has ceased.
            stop_group(process)
            process.stdout.close()
            stream.flush()
            os.fsync(stream.fileno())
    return dict(exit=process.returncode, wall_seconds=time.monotonic() - started,
                bytes=count, log_sha256=sha(log), containment_failure=failure)


def run_session(output, pins, observe, execute, clock=time.monotonic, prepare=lambda: None):
    output = Path(output)
    output.mkdir()  # Parent exists. A second entry, even after crash, refuses.
    started = clock()
    initial = None
    entries = []
    attempted = 0
    error = None
    write_new(output / "intent.json", dict(schema="local-single-session-074-v1",
              source=pins["source"], schedule=[[l, n] for l in LANES for n in range(3)],
              child_timeout_seconds=1200, kill_after_seconds=5,
              child_containment_seconds=CHILD_SECONDS, session_seconds=SESSION_SECONDS,
              max_log_bytes=MAX_LOG_BYTES, original_windows_seconds=[300, 60, 10], **SCOPE))
    try:
        prepare()
        initial = observe()
        write_new(output / "metadata.json", initial)
        validate_snapshot(initial, pins)
        for lane in LANES:
            for sample in range(3):
                require(clock() - started < SESSION_SECONDS - CHILD_SECONDS - 30,
                        "Insufficient bounded session budget; no dispatch")
                before = observe()
                write_new(output / f"{lane}-{sample}-before.json", before)
                validate_snapshot(before, pins, initial)
                prefix = output / f"{lane}-Profile-{sample}"
                write_new(prefix.with_suffix(".intent.json"), dict(lane=lane, sample=sample, **SCOPE))
                attempted += 1
                receipt = execute(lane, prefix.with_suffix(".log"),
                                  min(started + SESSION_SECONDS - 30, clock() + CHILD_SECONDS))
                entries.append(dict(lane=lane, sample=sample, **receipt))
                write_new(prefix.with_suffix(".exit.json"), entries[-1])
                after = observe()
                write_new(output / f"{lane}-{sample}-after.json", after)
                validate_snapshot(after, pins, initial)
                if receipt["exit"] != 0:
                    break  # Stop this lane, never replace its refusal.
    except BaseException as failure:
        error = type(failure).__name__ + ": " + str(failure)
    finally:
        try:
            final = observe()
            write_new(output / "postflight.json", final)
            validate_snapshot(final, pins, initial)
        except BaseException as failure:
            error = (error or "") + " Postflight: " + type(failure).__name__ + ": " + str(failure)
        # Missing result after SIGKILL/host loss is incomplete, never resumable.
        result = dict(source=pins["source"], profiles=entries, failure=error,
                      measured_profile_invocations=attempted,
                      elapsed_seconds=clock() - started,
                      schedule_completed=not error and len(entries) == 6 and
                      all(row["exit"] == 0 for row in entries),
                      full_matrix=False, **SCOPE)
        write_new(output / "result.json", result)
    return result


def verify_preparation(repo, pins):
    require(re.fullmatch(r"[0-9a-f]{40}", pins["source"]), "Invalid source")
    git(repo, "diff", "--exit-code", BASELINE, pins["source"], "--",
        "tools/long-run-supervisor-074", "crates", "Cargo.lock")
    root = repo / "docs/testing/performance/0.74/local-runs"
    prior = root / PRIOR
    require(sha(prior / "inventory.json") == PRIOR_HASH, "Prior seal changed")
    for row in read_json(prior / "inventory.json")["files"]:
        path = prior / row["path"]
        require(path.stat().st_size == row["bytes"] and sha(path) == row["sha256"], "Prior member changed")
    require(pins["bundle"] == read_json(prior / "context-Verified-Before.json")["bundle"], "Bundle pin drift")
    require(set(pins["images"]) == set(LANES) and all(
        re.fullmatch(r"[0-9a-f]{64}", v) for v in pins["images"].values()), "Image pin shape")
    for lane in LANES:
        receipt = read_json(prior / f"check-Verified-{lane}-exit.json")
        require(not receipt["failure"] and len(receipt["steps"]) == 11 and
                all(row["exit"] == 0 for row in receipt["steps"]), "Focused checks incomplete")
        git(repo, "diff", "--exit-code", receipt["source"], pins["source"], "--",
            "tools/long-run-supervisor-074", "crates", "Cargo.lock")
        artifacts = [json.loads(line) for line in (prior / f"check-Verified-{lane}-build.log").read_text().splitlines()
                     if line.startswith('{')]
        images = [a for a in artifacts if a.get("reason") == "compiler-artifact" and
                  a["target"]["name"] == "hydracache_long_run_supervisor_074" and
                  a["profile"]["test"] and a.get("executable")]
        require(len(images) == 1 and not images[0]["features"] and
                images[0]["executable"] == IMAGES[lane], "Not original default test image")
        fresh = read_json(pins["preparation"])[lane]
        require(fresh["source"] == pins["source"] and fresh["exit"] == 0 and
                fresh["image"] == IMAGES[lane] and fresh["sha256"] == pins["images"][lane], "Fresh image binding")
        fresh_log = Path(pins["preparation"]).parent / fresh["build_log"]
        require(sha(fresh_log) == fresh["build_log_sha256"], "Fresh build log changed")
        fresh_artifacts = [json.loads(line) for line in fresh_log.read_text().splitlines() if line.startswith('{')]
        fresh_images = [a for a in fresh_artifacts if a.get("reason") == "compiler-artifact" and
                        a["target"]["name"] == "hydracache_long_run_supervisor_074" and
                        a["profile"]["test"] and a.get("executable")]
        require(len(fresh_images) == 1 and not fresh_images[0]["features"] and
                fresh_images[0]["executable"] == IMAGES[lane], "Fresh default image receipt changed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--pins", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    pins = read_json(args.pins)
    require(args.source == pins["source"], "Source argument mismatch")
    require(args.output.parent.resolve() == repo / "docs/testing/performance/0.74/local-runs" and
            re.fullmatch(r"single-session-rounds-verified-20261021", args.output.name), "Only this new local packet")
    require(args.pins.resolve() == repo / "target/single-session-rounds-verified-20261021-pins.json", "Unexpected pins location")
    # Prechecks run inside the same WSL invocation, before fresh context/profiles.
    result = run_session(args.output, pins, lambda: snapshot(repo, args.pins),
                         lambda lane, log, deadline: capture(command(lane), child_env(repo), log, deadline),
                         prepare=lambda: verify_preparation(repo, pins))
    print(json.dumps(result, sort_keys=True), flush=True)
    return 0 if result["schedule_completed"] else 1


if __name__ == "__main__":
    def interrupted(signum, _frame):
        raise InterruptedError(f"Session signal {signum}; no continuation")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGHUP, interrupted)
    raise SystemExit(main())
