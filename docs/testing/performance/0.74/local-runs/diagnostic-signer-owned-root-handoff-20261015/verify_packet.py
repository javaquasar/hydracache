"""Unsigned historical consistency only: no execution, credential or ship authority."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
MANIFEST_SHA256 = "c4135c654c99fc49b42a2a7daa27b132508eecf563fc5dba8ed06694a1ec44b8"
SOURCE = "c30c0bc4cc417e44a38d7482dafaf651f3df3302"
TREE = "cfd4c3ca0fd95c4ea65e0139b5854bdcdcbfee88"
BASE = "ae1c3b7c3582fd8facbacb09dbc95654122ca378"
PREREG = "20ccd4b1f9c7fb89646683997376d5bed4a66e7e"
AMEND = "a8849dfdab4a10dc14dae57cbe693e5d86e2bece"

def require(ok, reason):
    if not ok:
        raise ValueError(reason)

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def lf(raw):
    return raw.replace(b"\r\n", b"\n")

def manifest(raw):
    require(digest(raw) == MANIFEST_SHA256, "manifest pin")
    data = json.loads(raw)
    require(data["source_commit"] == SOURCE and data["source_tree"] == TREE, "source identity")
    require(data["baseline"] == BASE and data["preregistration"] == PREREG and data["amendment"] == AMEND, "baseline/preregistration")
    require(all(type(value) is bool and not value for value in data["closed_claims"].values()), "closed claims")
    require(data["msrv_lint_executed"] is False, "MSRV lint unavailable")
    return data

def results(raw):
    return [[kind, int(passed), int(failed)] for kind, passed, failed in re.findall(
        r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", raw.decode())]

def capture(name, raw, expected):
    require(digest(raw) == expected["sha256"] and len(raw) == expected["bytes"], name + " bytes")
    require(expected["capture_complete"] is True, name + " incomplete")
    require(results(raw) == expected["results"], name + " results")
    for marker in expected["required_markers"]:
        require(raw.count(marker.encode()) == 1, name + " missing/duplicate marker")
    if name.startswith("current-"):
        require(raw.count(b"normal_root_handoff mode=") == 25, "25 fixed modes")
        require(raw.count(b"ack_close_exit=true") == 1, "one complete success")
        require(raw.count(b"success_claim=false") == 24, "24 refusal/cleanup controls")
        require(raw.count(b"attempted=1 sends=0") == 1, "failed send is attempted")
        require(raw.count(b"kind=loss automatic_death=true") == 5, "old stopped loss controls")
        require(b"extra_libm_root_handoff_modes=25" not in raw, "refusal is not current success")
    if name == "msrv.log":
        require(b"normal_root_handoff mode=" not in raw, "MSRV refusal not runtime acceptance")

def git(*args):
    env = os.environ.copy()
    dotgit = ROOT / ".git"
    if os.name != "nt" and dotgit.is_file():
        target = dotgit.read_text().strip().removeprefix("gitdir: ")
        if re.match(r"^[A-Za-z]:/", target):
            target = "/mnt/" + target[0].lower() + target[2:]
            env["GIT_DIR"] = target
            env["GIT_WORK_TREE"] = str(ROOT)
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)

def body(source, declaration):
    at = source.index(declaration)
    begin = source.index("{", at)
    depth = 0
    for end in range(begin, len(source)):
        depth += (source[end] == "{") - (source[end] == "}")
        if depth == 0:
            return source[begin + 1:end]
    raise ValueError("unterminated pinned function")

def old_owner_unchanged(before, after):
    for fn in ["fn raw_child(", "fn wait(", "fn enter(", "fn first_exec(", "fn advance("]:
        require(body(before, fn) == body(after, fn), "unchanged original function " + fn)

def installer_unchanged(before, after):
    for fn in ["fn install(", "fn entry(", "fn accept(", "fn decode(", "fn memory("]:
        require(body(before, fn) == body(after, fn), "unchanged installer function " + fn)
    old_child = body(before, "fn child(")
    recomposed = body(after, "fn install_child(") + body(after, "fn child(").replace("install_child();", "", 1)
    require(re.sub(r"\s+", "", old_child) == re.sub(r"\s+", "", recomposed), "same child installer and exercise")

def verify():
    data = manifest((PACKET / "manifest.json").read_bytes())
    require(git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE, "source tree")
    require(git("merge-base", "--is-ancestor", BASE, PREREG) == b"", "prereg ancestry")
    require(git("merge-base", "--is-ancestor", AMEND, SOURCE) == b"", "source ancestry")
    require(git("merge-base", "--is-ancestor", PREREG, AMEND) == b"", "amendment ancestry")
    for path, sha in data["sources"].items():
        require(digest(lf(git("show", SOURCE + ":" + path))) == sha, "source blob " + path)
    for path, sha in data["unchanged_sources"].items():
        require(digest(lf(git("show", SOURCE + ":" + path))) == sha, "unchanged blob " + path)
        require(digest(lf(git("show", BASE + ":" + path))) == sha, "baseline blob " + path)
    for path, sha in data["frozen_inputs"].items():
        require(digest((ROOT / path).read_bytes()) == sha, "frozen input " + path)
        require(digest(git("show", SOURCE + ":" + path)) == sha, "source frozen input " + path)
    prior = data["prior_negative"]
    require(digest((ROOT / prior["path"]).read_bytes()) == prior["sha256"], "prior negative bytes")
    require(digest(git("show", BASE + ":" + prior["path"])) == prior["sha256"], "prior negative baseline")
    path = "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_exec.rs"
    old_owner_unchanged(lf(git("show", BASE + ":" + path)).decode(), lf(git("show", SOURCE + ":" + path)).decode())
    path = "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_isolation.rs"
    installer_unchanged(lf(git("show", BASE + ":" + path)).decode(), lf(git("show", SOURCE + ":" + path)).decode())
    path = "tools/long-run-supervisor-074/tests/diagnostic_signer_root_probe.rs"
    require(body(lf(git("show", BASE + ":" + path)).decode(), "fn output(") == body(lf(git("show", SOURCE + ":" + path)).decode(), "fn output("), "outer guard unchanged")
    contract_path = "docs/testing/performance/0.74/diagnostic-signer-owned-root-handoff-contract.toml"
    c = tomllib.loads(git("show", SOURCE + ":" + contract_path).decode())
    for key, value in data["bounds"].items():
        require(type(c[key]) is int and c[key] == value, "contract bound " + key)
    for key in data["closed_claims"]:
        if key in c:
            require(c[key] is False, "contract admission " + key)
    require(set(p.name for p in PACKET.glob("*.log")) == set(data["captures"]), "capture set")
    for name, expected in data["captures"].items():
        capture(name, (PACKET / name).read_bytes(), expected)
    print("owned-root-handoff packet consistent; unsigned=true execution_attested=false admission=false")

if __name__ == "__main__":
    verify()
