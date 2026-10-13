"""Read-only early-attempt proof; never repair, reexecute or publish a journal."""
import hashlib
import json
import pathlib
import re
import subprocess
import tomllib

PACKET = pathlib.Path(__file__).resolve().parent
ROOT = PACKET.parents[5]


def require(condition, reason):
    if not condition:
        raise SystemExit("early-attempt packet refused: " + reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def normalized(path):
    return path.read_bytes().replace(b"\r\n", b"\n")


def git(ref, path):
    return subprocess.check_output(["git", "show", ref + ":" + path], cwd=ROOT)


def totals(text):
    return [list(map(int, values)) for values in re.findall(
        r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)]


m = json.loads((PACKET / "manifest.json").read_bytes())
require(m["schema_version"] == 1 and m["state"] == "local-original-early-attempt-only", "scope")
impl, baseline = m["implementation_source"], m["baseline_source"]
require(subprocess.check_output(["git", "rev-parse", impl + "^{tree}"], cwd=ROOT)
        .decode().strip() == m["implementation_tree"], "tree")
require(subprocess.run(["git", "merge-base", "--is-ancestor", m["preregistration_source"], impl],
                       cwd=ROOT, check=False).returncode == 0, "preregistration")
for path, digest in m["implementation_source_sha256"].items():
    raw = git(impl, path)
    require(sha(raw) == digest and normalized(ROOT / path) == raw, "source " + path)
require(sha(normalized(pathlib.Path(__file__))) == m["verifier_sha256"], "verifier")
for path, digest in m["unchanged_sources"].items():
    require(git(impl, path) == git(baseline, path)
            and sha(normalized(ROOT / path)) == digest, "unchanged original " + path)

contract_path = "docs/testing/performance/0.74/diagnostic-early-attempt-local-contract.toml"
contract = tomllib.loads((ROOT / contract_path).read_text())
require(contract == m["contract_pins"], "contract")
for key in [
    "new_record_schema", "global_nonce_protection", "fixed_progression_integration_implemented",
    "later_fixed_cells_implemented", "authenticated_peer_or_clock", "machine_power_loss_proved",
    "host_reboot_proved", "positive_host_enrollment_proved", "server_entry_wired",
    "worker_execution_allowed", "server_operation_performed", "qualification_allowed",
    "ship_allowed", "performance_claim_allowed",
]:
    require(contract[key] is False, "closed permission " + key)

qpath = "docs/testing/performance/0.74/qualification-manifest.toml"
qraw = (ROOT / qpath).read_bytes()
require(sha(qraw) == m["qualification_manifest_sha256"]
        == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc", "qualification")
q = tomllib.loads(qraw.decode())
require(not q["expensive_runs_enabled"] and q["candidate_source_sha"] == "UNRESOLVED", "qualification closed")
require(set(m["frozen_inputs_sha256"]) == {item["path"] for item in q["contract_inputs"]}, "input inventory")
for path, digest in m["frozen_inputs_sha256"].items():
    require(sha(git(baseline, path)) == digest == sha(normalized(ROOT / path)), "frozen " + path)
require(m["frozen_inputs_sha256"][q["contract_inputs"][3]["path"]]
        == "cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828",
        "inherited actual digest")
require(q["contract_inputs"][3]["sha256"]
        == "6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d",
        "inherited declared digest")

old = m["historical_packet"]
oldraw = (ROOT / old / "manifest.json").read_bytes()
require(oldraw == git(baseline, old + "/manifest.json")
        and sha(oldraw) == m["historical_manifest_sha256"], "historical manifest")
for record in json.loads(oldraw)["files"]:
    raw = (ROOT / old / record["name"]).read_bytes()
    require(raw == git(baseline, old + "/" + record["name"])
            and sha(raw) == record["sha256"], "historical capture")

records = {record["name"]: record for record in m["files"]}
require(len(records) == len(m["files"]) == 38, "capture count")
require({path.name for path in PACKET.glob("*.log")} == set(records), "raw inventory")
texts = {}
for name, record in records.items():
    raw = (PACKET / name).read_bytes()
    require(len(raw) == record["bytes"] and sha(raw) == record["sha256"], "raw " + name)
    require(record["expected_exit"] == record["observed_exit"] and record["command"], "exit/command " + name)
    text = raw.decode("utf-8")
    require(totals(text) == record["test_totals"], "totals " + name)
    if record["expected_exit"] == 0:
        require(not any(values[1] for values in record["test_totals"]), "failed tests " + name)
    texts[name] = text
require("unresolved import" in texts["02-red-absent-entry.log"], "absent API")
require("no method named" in texts["03-first-focused.log"], "fixture negative")
require("unused" in texts["06-linux-clippy-first.log"], "lint negative")
require("self.empty.fence.inspect" in texts["09-root-regression-first.log"], "contract negative")

stage_names = ["BeforeCreate", "Created", "PrefixWritten", "BytesWritten", "Sealed",
               "FileSynced", "DirectorySynced", "Acknowledging", "AfterReturn"]
expected_cases = {(stage, "SIGKILL", "9") for stage in stage_names} | {("AfterReturn", "control", "0")}
pattern = r"P1 case=attempt:(\w+) terminated=(SIGKILL|control) signal=(\d+) fresh_processes=2 retained=unchanged original_lock=released helper=terminal"
for repeat in range(1, 4):
    functional = f"21-functional-repeat-{repeat}.log"
    process = f"22-process-repeat-{repeat}.log"
    require(records[functional]["test_totals"] == [[12, 0, 0]], "functional repeat")
    for name in m["required_test_names"]:
        require(m["test_prefix"] + name + " ... ok" in texts[functional], "named case " + name)
    require(records[process]["test_totals"] == [[1, 0, 0]], "process repeat")
    observed = re.findall(pattern, texts[process])
    require(len(observed) == 10 and set(observed) == expected_cases, "signal matrix " + process)
    require("seed=0x074d stages=9" in texts[process], "seed")
    require(m["manual_test_full_name"] in texts[process], "manual exact case")

for name, expected in [
    ("01-baseline.log", [15, 0, 0]),
    ("10-windows-regression.log", [204, 0, 0]),
    ("16-root-regression-corrected.log", [163, 0, 0]),
    ("24-linux-regression.log", [619, 0, 5]),
    ("25-linux-doctests.log", [41, 0, 0]),
    ("23-all-process-regression.log", [4, 0, 0]),
]:
    values = [sum(row[i] for row in records[name]["test_totals"]) for i in range(3)]
    require(values == expected, "regression " + name)
all_process = texts["23-all-process-regression.log"]
require(len(re.findall(pattern, all_process)) == 10, "new phase regression")
require(len(re.findall(r"P0 case=.* terminated=SIGKILL signal=9 ", all_process)) == 24, "old signal cases")
require(len(re.findall(r"P0 case=.* terminated=control signal=0 ", all_process)) == 2, "old controls")
require("contract input digest mismatch" in texts["30-qualification-refusal.log"], "qualification refusal")
require("ship admission is closed" in texts["31-ship-refusal.log"], "ship refusal")

registry_path = "docs/testing/release-evidence/0.74.toml"
registry = tomllib.loads((ROOT / registry_path).read_text())
original_registry = tomllib.loads(git(impl, registry_path).decode())
w11 = next(item for item in registry["work_item"] if item["id"] == "W11")
original_w11 = next(item for item in original_registry["work_item"] if item["id"] == "W11")
relative = PACKET.relative_to(ROOT).as_posix()
original_w11["required_artifacts"].extend(relative + "/" + name
                                        for name in ["manifest.json", "verify-packet.py", "README.md"])
require(registry == original_registry, "registry drift")
registered = {(item["source"], item["function"]) for item in w11["required_tests"]}
for name in m["required_test_names"]:
    require(("tools/long-run-supervisor-074/src/diagnostic_early_attempt_tests.rs", name)
            in registered, "W11 named case")
require(m["manual_gate_id"] in w11["gated_gate_ids"], "W11 manual gate")
gates = tomllib.loads((ROOT / "docs/testing/gated-test-registry.toml").read_text())
gate = next(item for item in gates["gate"] if item["id"] == m["manual_gate_id"])
require(gate["tier"] == "manual" and not gate["ship_mandatory"]
        and gate["command"]["platform"] == "linux"
        and m["manual_test_full_name"] in gate["command"]["args"], "manual registry")
print("early-attempt packet: OK; 3 x 9 SIGKILL + control; no adoption/start/qualification")
