"""Read-only exact-source check. No execution, repair or evidence publication."""
import hashlib
import json
import pathlib
import re
import subprocess
import tomllib

PACKET = pathlib.Path(__file__).resolve().parent
ROOT = PACKET.parents[5]


def require(condition, message):
    if not condition:
        raise SystemExit("transition packet refused: " + message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def git(ref, path):
    return subprocess.check_output(["git", "show", ref + ":" + path], cwd=ROOT)


def text_bytes(path):
    return path.read_bytes().replace(b"\r\n", b"\n")


def totals(text):
    return [list(map(int, values)) for values in re.findall(
        r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)]


m = json.loads((PACKET / "manifest.json").read_bytes())
require(m["schema_version"] == 1, "schema")
require(m["state"] == "local-model-adjacent-transition-only", "scope")
impl = m["implementation_source"]
baseline = m["baseline_source"]
require(subprocess.check_output(["git", "rev-parse", impl + "^{tree}"],
                               cwd=ROOT).decode().strip() == m["implementation_tree"], "tree")
require(subprocess.run(["git", "merge-base", "--is-ancestor", baseline, impl],
                       cwd=ROOT, check=False).returncode == 0, "preregistration ancestry")
require(m["preregistration_source"] == baseline, "preregistration identity")
require(git(baseline, "tools/long-run-supervisor-074/src/diagnostic_lease.rs")
        == git(m["starting_source"], "tools/long-run-supervisor-074/src/diagnostic_lease.rs"),
        "unmodified model baseline")
require(sha(text_bytes(pathlib.Path(__file__))) == m["verifier_sha256"], "verifier hash")
for path, digest in m["implementation_source_sha256"].items():
    raw = git(impl, path)
    require(sha(raw) == digest and text_bytes(ROOT / path) == raw, "source " + path)
registry_path = "docs/testing/release-evidence/0.74.toml"
require(sha(git(impl, registry_path)) == m["implementation_registry_sha256"],
        "implementation registry")
for path, digest in m["unchanged_sources"].items():
    require(git(impl, path) == git(baseline, path), "changed original " + path)
    require(sha(text_bytes(ROOT / path)) == digest, "original drift " + path)

contract_path = "docs/testing/performance/0.74/diagnostic-transition-local-contract.toml"
contract = tomllib.loads((ROOT / contract_path).read_text())
require(contract == m["contract_pins"], "contract pins")
for key in [
    "new_durable_format", "fixed_state_progression_implemented", "later_fixed_cells_implemented",
    "early_attempt_nonce_protection_implemented", "authenticated_peer_or_clock",
    "server_entry_wired", "worker_execution_allowed", "server_operation_performed",
    "qualification_allowed", "ship_allowed", "performance_claim_allowed",
]:
    require(contract[key] is False, "closed permission " + key)

qpath = "docs/testing/performance/0.74/qualification-manifest.toml"
qraw = (ROOT / qpath).read_bytes()
require(sha(qraw) == m["qualification_manifest_sha256"]
        == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc", "qualification")
q = tomllib.loads(qraw.decode())
require(not q["expensive_runs_enabled"] and q["candidate_source_sha"] == "UNRESOLVED",
        "qualification remains closed")
require(set(m["frozen_inputs_sha256"]) == {item["path"] for item in q["contract_inputs"]},
        "frozen input inventory")
for path, digest in m["frozen_inputs_sha256"].items():
    require(sha(git(baseline, path)) == digest == sha(text_bytes(ROOT / path)),
            "frozen input " + path)
plan_path = q["contract_inputs"][3]["path"]
require(m["frozen_inputs_sha256"][plan_path]
        == "cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828",
        "inherited actual plan digest")
require(q["contract_inputs"][3]["sha256"]
        == "6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d",
        "inherited declared plan digest")

old = m["historical_packet"]
oldraw = (ROOT / old / "manifest.json").read_bytes()
require(sha(oldraw) == m["historical_manifest_sha256"]
        and oldraw == git(baseline, old + "/manifest.json"), "historical manifest")
history = json.loads(oldraw)
for record in history["files"]:
    path = ROOT / old / record["name"]
    require(path.read_bytes() == git(baseline, old + "/" + record["name"])
            and sha(path.read_bytes()) == record["sha256"], "historical raw " + record["name"])

records = {record["name"]: record for record in m["files"]}
require(len(records) == len(m["files"]) == 35, "raw capture inventory")
require({path.name for path in PACKET.glob("*.log")} == set(records), "extra/missing raw captures")
texts = {}
for name, record in records.items():
    raw = (PACKET / name).read_bytes()
    require(len(raw) == record["bytes"] and sha(raw) == record["sha256"], "raw hash " + name)
    require(record["observed_exit"] == record["expected_exit"], "exit " + name)
    require(record["command"] and record["platform"] in ("windows", "linux"), "command " + name)
    text = raw.decode("utf-8")
    require(totals(text) == record["test_totals"], "test totals " + name)
    if record["expected_exit"] == 0:
        require(not any(values[1] for values in record["test_totals"]), "failed tests " + name)
    texts[name] = text

require("unresolved import" in texts["02-red-missing-validator.log"], "absent-entry red")
require("Reserved->Terminal" in texts["03-first-focused.log"]
        and "FAILED" in texts["03-first-focused.log"], "negative fixture")
require("no test target named" in texts["11-root-regression.log"], "negative invocation")
for repeat in range(1, 4):
    name = f"22-linux-focused-repeat-{repeat}.log"
    require(records[name]["test_totals"] == [[8, 0, 0]], "focused repeat")
    require("transition seed=0x074e histories=64 accepted=1280 refused=2560" in texts[name],
            "seeded totals " + name)
    for function in m["required_test_names"]:
        require(m["test_name_prefix"] + function + " ... ok" in texts[name], "test " + function)
for name, expected in [
    ("01-baseline-windows.log", [18, 0, 0]),
    ("09-windows-package.log", [204, 0, 0]),
    ("12-root-corrected.log", [162, 0, 0]),
    ("23-linux-final-regression.log", [607, 0, 4]),
    ("24-linux-doctests.log", [38, 0, 0]),
]:
    observed = [sum(values[i] for values in records[name]["test_totals"]) for i in range(3)]
    require(observed == expected, "regression " + name)
require("concurrent_campaign_and_diagnostic_admission_has_exactly_one_winner ... ok"
        in texts["23-linux-final-regression.log"], "existing concurrency proof")
require("contract input digest mismatch" in texts["28-qualification-refusal.log"],
        "qualification refusal")
require("ship admission is closed" in texts["29-ship-refusal.log"], "ship refusal")
registry = tomllib.loads((ROOT / registry_path).read_text())
w11 = next(item for item in registry["work_item"] if item["id"] == "W11")
for path in [
    contract_path, "docs/testing/performance/0.74/diagnostic-transition-local-design.md",
    "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
    "tools/long-run-supervisor-074/src/diagnostic_transition_tests.rs",
]:
    require(path in w11["required_sources"], "W11 source " + path)
registered = {(item["source"], item["function"]) for item in w11["required_tests"]}
for function in m["required_test_names"]:
    require(("tools/long-run-supervisor-074/src/diagnostic_transition_tests.rs", function)
            in registered, "W11 test " + function)
require(("crates/xtask/tests/performance_contract_074.rs",
         "diagnostic_transition_contract_keeps_fixed_execution_closed") in registered,
        "W11 closed contract test")
relative = PACKET.relative_to(ROOT).as_posix()
for name in ["manifest.json", "verify-packet.py", "README.md"]:
    require(relative + "/" + name in w11["required_artifacts"], "W11 artifact " + name)
original_registry = tomllib.loads(git(impl, registry_path).decode())
original_w11 = next(item for item in original_registry["work_item"] if item["id"] == "W11")
original_w11["required_artifacts"].extend(
    relative + "/" + name for name in ["manifest.json", "verify-packet.py", "README.md"])
require(registry == original_registry, "unreviewed registry drift")
print("transition packet: OK; local model only; fixed lifecycle/execution/qualification closed")
