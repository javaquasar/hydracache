"""Read-only exact-source evidence verification; no build, host or replay."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
BASELINE = "8549010a649a4a7394071a3dbaff67c794fc6a31"
IMPLEMENTATION = "2d446ec5b8a11b0ecd64ec813faaddaa91c9afa7"
TREE = "ce2adcc6ae395089bcb2214cca46cf2bec9c7b3a"
PREREGISTRATION = "718707058958b8f0ce81e0d6716f9965f92b9708"
CODE_FILES = [
  "tools/long-run-supervisor-074/src/diagnostic_signed_history.rs",
  "tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs",
  "tools/long-run-supervisor-074/src/diagnostic_intent_journal_tests.rs",
  "crates/xtask/tests/performance_contract_074.rs",
  "docs/testing/performance/0.74/diagnostic-signed-history-local-contract.toml",
  "docs/testing/performance/0.74/diagnostic-signed-history-local-design.md",
  "docs/COMPAT.md",
  "docs/GATES.md",
  ".gitattributes"
]
HISTORICAL = "docs/testing/performance/0.74/local-runs/diagnostic-attempt-publication-20261013"
MANIFEST_HASH = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
PREFIX = "diagnostic_output_preparation::intent_journal::tests::"

def require(condition, message):
    if not condition:
        raise SystemExit("FAIL: " + message)

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def lf(raw):
    return raw.replace(b"\r\n", b"\n")

def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args])

def totals(text):
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text)
    require(bool(rows), "missing successful test result")
    return tuple(sum(int(row[i]) for row in rows) for i in (0, 1))

m = json.loads((PACKET / "manifest.json").read_bytes())
require(m["schema_version"] == 1 and m["state"] == "local-supplied-byte-inspection-only", "scope")
require(m["baseline_source"] == BASELINE and m["preregistration_source"] == PREREGISTRATION, "baseline/preregistration")
require(m["implementation_source"] == IMPLEMENTATION and m["implementation_tree"] == TREE, "implementation pins")
require(git("rev-parse", IMPLEMENTATION + "^{tree}").decode().strip() == TREE, "Git tree")
git("merge-base", "--is-ancestor", BASELINE, PREREGISTRATION)
git("merge-base", "--is-ancestor", PREREGISTRATION, IMPLEMENTATION)
git("merge-base", "--is-ancestor", IMPLEMENTATION, "HEAD")
require(set(m["implementation_source_sha256"]) == set(CODE_FILES), "source inventory")
for path, expected in m["implementation_source_sha256"].items():
    committed = git("show", IMPLEMENTATION + ":" + path)
    require(digest(lf(committed)) == expected, "committed source " + path)
    require(digest(lf((ROOT / path).read_bytes())) == expected, "current source " + path)
for path, expected in m["unchanged_sources"].items():
    require(digest(lf(git("show", BASELINE + ":" + path))) == expected, "baseline unchanged source " + path)
    require(digest(lf((ROOT / path).read_bytes())) == expected, "unchanged source " + path)
contract = tomllib.loads((ROOT / "docs/testing/performance/0.74/diagnostic-signed-history-local-contract.toml").read_text())
require(contract == m["contract_pins"], "complete contract")
for key in (
    "filesystem_or_process_operation", "history_is_completion_proof",
    "recovered_history_is_runtime_acknowledgement", "original_object_ownership_proved",
    "later_fixed_cells_implemented", "full_fixed_lifecycle_implemented", "global_nonce_protection",
    "authenticated_peer_or_clock", "host_reboot_proved", "positive_host_enrollment_proved",
    "server_entry_wired", "worker_execution_allowed", "server_operation_performed",
    "qualification_allowed", "ship_allowed", "performance_claim_allowed",
):
    require(contract[key] is False, "closed boundary " + key)
require(contract["minimum_reserved_revision_gap"] == 3 and contract["max_retained_cells"] == 4, "fixed limits")
require(contract["test_seed"] == "0x074f" and contract["targeted_repetitions"] == 3, "fixed repeats/seed")
require(digest(lf(Path(__file__).read_bytes())) == m["verifier_sha256"], "verifier hash")
files = m["files"]
require(len(files) == 33, "raw log count")
require(len({f["path"] for f in files}) == len(files), "unique raw files")
require({p.name for p in PACKET.glob("*.log")} == {f["path"] for f in files} | {f["path"] for f in m["supplemental_checks"]}, "exact capture inventory")
logs = {}
for f in files:
    require(Path(f["path"]).name == f["path"], "log leaf")
    require(f["platform"] in ("linux", "windows") and bool(f["command"]), "command/platform")
    require(f["observed_exit_code"] == f["expected_exit_code"], "exit " + f["id"])
    raw = (PACKET / f["path"]).read_bytes()
    require(len(raw) == f["bytes"] and digest(raw) == f["sha256"], "raw integrity " + f["id"])
    logs[f["id"]] = raw.decode("utf-8", errors="replace")
supplement = m["supplemental_checks"]
require({f["path"] for f in supplement} == {"windows-build-lock-negative.log", "final-release-evidence.log"}, "supplement inventory")
for f in supplement:
    raw = (PACKET / f["path"]).read_bytes()
    require(len(raw) == f["bytes"] and digest(raw) == f["sha256"], "supplement integrity")
    require(f["observed_exit_code"] == f["expected_exit_code"], "supplement exit")
    text = raw.decode("utf-8", errors="replace")
    if f["capture_kind"] == "tool-returned-output":
        require(f["expected_exit_code"] == 1 and "os error 5" in text, "retained Windows build lock")
    else:
        require(f["capture_kind"] == "redirected-process-output" and totals(text) == (13, 0), "serial release-evidence check")
require(totals(logs["baseline-journal"]) == (19, 0), "baseline journal")
require("E0433" in logs["red-missing-history-entry"] and "unresolved module" in logs["red-missing-history-entry"], "red missing API")
require(next(f for f in files if f["id"] == "red-missing-history-entry")["expected_exit_code"] == 101, "red exit")
require(totals(logs["history-first"]) == (9, 0), "initial pre-addition focus")
for repeat in range(1, 4):
    text = logs["history-repeat-" + str(repeat)]
    require(totals(text) == (11, 0), "repeat total")
    for name in m["required_test_names"]:
        require("test " + PREFIX + name + " ... ok" in text, "named repeat case " + name)
    require("signed history seed=0x074f legal=64 signed_mutations=64; inspection is not completion" in text, "seed matrix")
for name, expected in (
    ("linux-regression", (642, 7)), ("linux-regression-final", (644, 7)),
    ("windows-regression", (70, 0)), ("windows-full-regression", (204, 0)),
    ("root-contract-regression", (165, 0)),
    ("linux-rustdoc", (44, 0)),
):
    require(totals(logs[name]) == expected, "regression " + name)
require("diagnostic_signed_history_is_data_only_not_fixed_progression ... ok" in logs["root-contract-regression"], "root contract")
require("FAIL: regression windows-regression" in logs["packet-verifier-scope-negative"], "retained scoped/full count correction")
require(next(f for f in files if f["id"] == "packet-verifier-scope-negative")["expected_exit_code"] == 1, "negative verifier exit")
require("KeyError: 'artifacts'" in logs["packet-verifier-registry-negative"], "retained registry field correction")
require(next(f for f in files if f["id"] == "packet-verifier-registry-negative")["expected_exit_code"] == 1, "registry verifier negative exit")
require("signed_history::SignedHistoryObservation" in logs["linux-rustdoc"], "no-start compile fail")
require("ship admission is closed" in logs["ship-refusal"], "ship remains closed")
require("digest" in logs["qualification-refusal"].lower(), "inherited digest refusal")
for name in ("ship-refusal", "qualification-refusal"):
    require(next(f for f in files if f["id"] == name)["expected_exit_code"] == 1, "expected refusal")
require(not Path(m["qualification_output_must_not_exist"]).exists(), "qualification produced no output")
qpath = ROOT / "docs/testing/performance/0.74/qualification-manifest.toml"
require(digest(qpath.read_bytes()) == MANIFEST_HASH == m["qualification_manifest_sha256"], "qualification raw manifest")
q = tomllib.loads(qpath.read_text())
require(len(q["contract_inputs"]) == 8 and q["candidate_source_sha"] == "UNRESOLVED" and not q["expensive_runs_enabled"], "qualification closed")
require(set(m["frozen_inputs_sha256"]) == {f["path"] for f in q["contract_inputs"]}, "frozen inventory")
for path, expected in m["frozen_inputs_sha256"].items():
    require(digest(lf(git("show", BASELINE + ":" + path))) == expected, "frozen baseline " + path)
    require(digest(lf((ROOT / path).read_bytes())) == expected, "frozen current " + path)
plan = next(f for f in q["contract_inputs"] if f["path"].startswith("docs/plans/"))
require(plan["sha256"] == "6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d", "inherited declared plan")
require(m["frozen_inputs_sha256"][plan["path"]] == "cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828", "inherited actual plan")
old_manifest = json.loads((ROOT / HISTORICAL / "manifest.json").read_bytes())
for name in ["manifest.json"] + [f["name"] for f in old_manifest["files"]]:
    path = HISTORICAL + "/" + name
    require((ROOT / path).read_bytes() == git("show", BASELINE + ":" + path), "historical raw " + name)
registry_path = "docs/testing/release-evidence/0.74.toml"
expected_registry = tomllib.loads(git("show", IMPLEMENTATION + ":" + registry_path).decode())
actual_registry = tomllib.loads((ROOT / registry_path).read_text())
w11 = next(w for w in expected_registry["work_item"] if w["id"] == "W11")
w11["required_artifacts"].extend("docs/testing/performance/0.74/local-runs/diagnostic-signed-history-20261013/" + name for name in ("README.md", "manifest.json", "verify_packet.py"))
require(actual_registry == expected_registry, "W11 scoped evidence-only registry addition")
print("PASS: anchored signed history inspection; 33 primary logs and two supplemental captures; three exact-source repeats; no execution or qualification")

