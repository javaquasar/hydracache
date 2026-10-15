"""Unsigned historical consistency; never execution attestation or admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "97d9304f162a548ba0181cb4334960598b5f1cac"
TREE = "85a1857c1a307d05150440279b5b67c9ef5cc717"
BASE = "89f81f454bfc5fcdb2d87619337d141a1dd95452"
PREREG = "22b92daf03672779599f4004eff425dd2f04c2c7"
AMEND = "4887964a0e675c742dc13e8fa1ba04efc4cc0562"
MANIFEST_SHA256 = "671f39d55ff719137066f6f15870191c4de0af10cb69543455c534e6b111bf4f"
CUTS = ("installed", "ready", "receive", "delivered", "acked", "closed")
CLOSED = (
    "fresh_creator_cli_exec_proved", "pre_arm_loss_proved", "running_child_loss_proved",
    "creator_thread_only_loss_proved", "full_normal_entry_creator_loss_proved",
    "overlapping_broad_load_robustness_proved", "credential_handoff_authenticated",
    "runtime_custody_authenticated", "original_worker_start_authenticated",
    "all_writers_revoked", "real_credentials_or_signing", "completion_or_later_cells_enabled",
    "server_or_github_mutated", "qualification_allowed", "ship_allowed", "performance_claim_allowed",
)

def require(ok, why):
    if not ok:
        raise ValueError(why)

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def lf(raw):
    return raw.replace(b"\r\n", b"\n")

def manifest(raw):
    require(digest(raw) == MANIFEST_SHA256, "manifest pin")
    data = json.loads(raw)
    require(data["source_commit"] == SOURCE and data["source_tree"] == TREE, "source identity")
    require(data["baseline"] == BASE and data["preregistration"] == PREREG
            and data["amendment"] == AMEND, "baseline/preregistration")
    require(set(data["closed_claims"]) == set(CLOSED), "closed claim set")
    require(all(type(v) is bool and not v for v in data["closed_claims"].values()), "closed claims")
    require(data["msrv_linux_lint_executed"] is False, "MSRV Linux lint unavailable")
    require(data["msrv_windows_lint_executed"] is True, "MSRV Windows lint")
    require(data["msrv_windows_lint_passed"] is False, "retained MSRV macro lint failure")
    require(data["current_toolchain_scoped_checks_passed"] is True, "current scoped checks")
    for key in ("all_checks_passed", "full_release_gate_executed", "execution_attested",
                "clean_checkout_execution_attested"):
        require(data[key] is False, "not a full green or attestation receipt")
    return data

def results(raw):
    return [[kind, int(passed), int(failed)] for kind, passed, failed in re.findall(
        r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", raw.decode())]

def transfer_markers():
    lines = []
    for kind, cuts in [("loss", CUTS), ("healthy", CUTS), ("no-exitkill", ("installed", "delivered"))]:
        for cut in cuts:
            sent = int(cut in ("delivered", "acked", "closed"))
            automatic = str(kind == "loss").lower()
            complete = str(kind == "healthy").lower()
            lines.append(f"normal_root_transfer kind={kind} cut={cut} sends_at_cut={sent} "
                         f"automatic_death={automatic} ack_close_exit={complete} "
                         "original_reaped=true eof=true unrelated_alive=true admission=false")
    return lines

def capture(name, raw, expected):
    require(digest(raw) == expected["sha256"] and len(raw) == expected["bytes"], name + " bytes")
    require(expected["capture_complete"] is (name != "baseline-process-tail.log"), name + " completeness")
    require(results(raw) == expected["results"], name + " results")
    for marker in expected["required_markers"]:
        require(raw.count(marker.encode()) == 1, name + " marker")
    if name.startswith("current-"):
        require(results(raw) == [["ok", n, 0] for n in (43, 5, 7, 1, 7)], "63 final tests")
        actual = [line for line in raw.decode().splitlines() if line.startswith("normal_root_transfer ")]
        require(actual == transfer_markers(), "14 classified transfer controls")
        require(raw.count(b"normal_root_handoff mode=") == 25, "old 25 handoff modes")
        require(raw.count(b"success_claim=false") == 24, "old refusal controls")
        require(raw.count(b"attempted=1 sends=0") == 1, "sticky failed send")
        require(b"extra_libm_root_transfer_modes" not in raw, "runtime not refusal")
    if name == "msrv.log":
        require(results(raw) == [["ok", n, 0] for n in (43, 5, 7, 1, 7)], "63 MSRV tests")
        require(b"normal_root_transfer " not in raw and b"normal_root_handoff mode=" not in raw,
                "MSRV refusal not runtime support")
        require(raw.count(b"extra_libm_root_transfer_modes=14 refused=true") == 1, "14 attributed refusals")

def git(*args):
    env = os.environ.copy()
    if os.name != "nt" and (ROOT / ".git").is_file():
        target = (ROOT / ".git").read_text().strip().removeprefix("gitdir: ")
        if re.match(r"^[A-Za-z]:/", target):
            env["GIT_DIR"] = "/mnt/" + target[0].lower() + target[2:]
            env["GIT_WORK_TREE"] = str(ROOT)
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)

def body(source, declaration):
    begin = source.index("{", source.index(declaration))
    depth = 0
    for end in range(begin, len(source)):
        depth += (source[end] == "{") - (source[end] == "}")
        if depth == 0:
            return source[begin + 1:end]
    raise ValueError("unterminated pinned function")

def verify():
    data = manifest((PACKET / "manifest.json").read_bytes())
    require(git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE, "source tree")
    for before, after in [(BASE, PREREG), (PREREG, AMEND), (AMEND, SOURCE)]:
        require(git("merge-base", "--is-ancestor", before, after) == b"", "prereg ancestry")
    for path, sha in data["sources"].items():
        require(digest(lf(git("show", SOURCE + ":" + path))) == sha, "source " + path)
    for path, sha in data["unchanged_sources"].items():
        for revision in (BASE, SOURCE):
            require(digest(lf(git("show", revision + ":" + path))) == sha, "unchanged " + path)
    for path, sha in data["frozen_inputs"].items():
        require(digest((ROOT / path).read_bytes()) == sha, "frozen checkout " + path)
        require(digest(git("show", SOURCE + ":" + path)) == sha, "frozen source " + path)
    for prior in data["prior_negatives"]:
        require(digest((ROOT / prior["path"]).read_bytes()) == prior["sha256"], "prior negative")
    for path, declarations in data["unchanged_functions"].items():
        before = lf(git("show", BASE + ":" + path)).decode()
        after = lf(git("show", SOURCE + ":" + path)).decode()
        for declaration in declarations:
            require(body(before, declaration) == body(after, declaration), "original function " + declaration)
    c = tomllib.loads(git("show", SOURCE + ":docs/testing/performance/0.74/diagnostic-signer-owned-root-handoff-loss-contract.toml").decode())
    for key, value in data["bounds"].items():
        require(type(c[key]) is int and c[key] == value, "bound " + key)
    for key in CLOSED:
        require(c[key] is False, "contract closed " + key)
    require(set(p.name for p in PACKET.glob("*.log")) == set(data["captures"]), "capture set")
    for name, expected in data["captures"].items():
        capture(name, (PACKET / name).read_bytes(), expected)
    print("owned-root-handoff-loss packet consistent; unsigned=true execution_attested=false admission=false")

if __name__ == "__main__":
    verify()
