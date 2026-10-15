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
MANIFEST_SHA256 = "6a4d6fe860b09b88cc73716351ecb6d0f58ee3aa849e748a1daf24e41f0a23e9"
SOURCE = "1f399e0a7040d2a0733825aed2fd13fc6caf85a4"
TREE = "591821cf65dc780ce52cfa00f2d690a146cec995"
BASE = "72b4ba1e7bbaac5d05230c3e55d2a5e6dfb0ee41"
PREREG = "9fe69db4d262e16d6b3da48410c95899d652936f"

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
    require(data["baseline"] == BASE and data["preregistration"] == PREREG, "baseline/preregistration")
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
        require(raw.count(b"kind=loss automatic_death=true") == 5, "five automatic cuts")
        require(raw.count(b"kind=healthy automatic_death=false") == 2, "two healthy controls")
        require(raw.count(b"kind=negative automatic_death=false") == 1, "one negative control")
        require(b"extra_libm_root_creator_loss_modes=8" not in raw, "refusal is not current success")
    if name == "msrv.log":
        require(b"normal_root_creator_loss mode=" not in raw, "MSRV refusal not runtime acceptance")

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

def old_owner_unchanged(before, after):
    module = '#[path = "diagnostic_signer_owned_root_loss.rs"]\nmod loss;\n'
    entry = 'pub(super) fn creator_loss(mode: &str) -> io::Result<()> {\n    loss::run(mode)\n}\n'
    require(after.count(module) == 1 and after.count(entry) == 1, "loss entry exact")
    require(after.replace(module, "").replace(entry, "") == before, "ordinary Owner/raw child changed")

def verify():
    data = manifest((PACKET / "manifest.json").read_bytes())
    require(git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE, "source tree")
    require(git("merge-base", "--is-ancestor", BASE, PREREG) == b"", "prereg ancestry")
    require(git("merge-base", "--is-ancestor", PREREG, SOURCE) == b"", "source ancestry")
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
    contract_path = "docs/testing/performance/0.74/diagnostic-signer-owned-root-loss-contract.toml"
    c = tomllib.loads(git("show", SOURCE + ":" + contract_path).decode())
    for key, value in data["bounds"].items():
        require(type(c[key]) is int and c[key] == value, "contract bound " + key)
    for key in data["closed_claims"]:
        if key in c:
            require(c[key] is False, "contract admission " + key)
    require(set(p.name for p in PACKET.glob("*.log")) == set(data["captures"]), "capture set")
    for name, expected in data["captures"].items():
        capture(name, (PACKET / name).read_bytes(), expected)
    print("owned-root-loss packet consistent; unsigned=true execution_attested=false admission=false")

if __name__ == "__main__":
    verify()
