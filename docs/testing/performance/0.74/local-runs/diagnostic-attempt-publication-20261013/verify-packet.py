"""Read-only exact-source owned-transfer proof. No repair, resume or execution."""
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
        raise SystemExit("attempt-transfer packet refused: " + reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def normalized(path):
    return path.read_bytes().replace(b"\r\n", b"\n")


def git(ref, path):
    return subprocess.check_output(["git", "show", ref + ":" + path], cwd=ROOT)


def totals(text):
    return [list(map(int, row)) for row in re.findall(
        r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)]


m = json.loads((PACKET / "manifest.json").read_bytes())
require(m["schema_version"] == 1 and m["state"] == "local-owned-attempt-transfer-only", "scope")
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

contract_path = "docs/testing/performance/0.74/diagnostic-attempt-publication-local-contract.toml"
c = tomllib.loads((ROOT / contract_path).read_text())
require(c == m["contract_pins"], "contract")
for key in [
    "new_durable_schema", "new_filename", "later_fixed_cells_implemented",
    "full_fixed_lifecycle_implemented", "global_nonce_protection", "authenticated_peer_or_clock",
    "machine_power_loss_proved", "host_reboot_proved", "positive_host_enrollment_proved",
    "server_entry_wired", "worker_execution_allowed", "server_operation_performed",
    "qualification_allowed", "ship_allowed", "performance_claim_allowed",
]:
    require(c[key] is False, "closed permission " + key)
require(c["targeted_repetitions"] == 3 and c["fresh_processes_per_repeat"] == 52, "matrix contract")

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
        == "cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828", "actual inherited plan")
require(q["contract_inputs"][3]["sha256"]
        == "6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d", "declared inherited plan")

old = m["historical_packet"]
oldraw = (ROOT / old / "manifest.json").read_bytes()
require(oldraw == git(baseline, old + "/manifest.json")
        and sha(oldraw) == m["historical_manifest_sha256"], "historical manifest")
for record in json.loads(oldraw)["files"]:
    raw = (ROOT / old / record["name"]).read_bytes()
    require(raw == git(baseline, old + "/" + record["name"])
            and sha(raw) == record["sha256"], "historical capture " + record["name"])

records = {record["name"]: record for record in m["files"]}
require(len(records) == len(m["files"]) == 45, "capture count")
require({path.name for path in PACKET.glob("*.log")} == set(records), "raw inventory")
texts = {}
for name, record in records.items():
    raw = (PACKET / name).read_bytes()
    require(len(raw) == record["bytes"] and sha(raw) == record["sha256"], "raw " + name)
    require(record["expected_exit"] == record["observed_exit"]
            and record["command"] and record["platform"] in ("linux", "windows"), "exit/command")
    text = raw.decode("utf-8")
    require(totals(text) == record["test_totals"], "totals " + name)
    if record["expected_exit"] == 0:
        require(not any(row[1] for row in record["test_totals"]), "failed tests " + name)
    texts[name] = text
require("unresolved import" in texts["02-red-entry.log"], "absent API")
require("256 bytes" in texts["05-linux-clippy.log"] and "very large" in texts["05-linux-clippy.log"], "lint negative")
require("only_state(self.leaf()?)?" in texts["08-root-regression.log"], "legacy contract negative")

publication_stages = ["BeforeCreate", "StateCreated", "PrefixWritten", "BytesWritten",
                      "FileSynced", "DirectorySynced", "DocumentBound", "Acknowledging", "AfterReturn"]
consumption_stages = ["BeforeRecord", "RecordCreated", "RecordPrefixWritten", "RecordBytesWritten",
                      "RecordSealed", "RecordSynced", "BeforeCommit", "CommitCreated",
                      "CommitPrefixWritten", "CommitBytesWritten", "CommitSealed",
                      "CommitSynced", "DirectorySynced", "Acknowledging", "AfterReturn"]


def signals(text, phase):
    return re.findall(r"P1 case=" + phase + r":(\w+) terminated=(SIGKILL|control) signal=(\d+) fresh_processes=2 retained=unchanged original_lock=released helper=terminal", text)


for repeat in range(1, 4):
    name = f"25-functional-repeat-{repeat}.log"
    require(records[name]["test_totals"] == [[14, 0, 0]], "functional repeat")
    for function in m["required_test_names"]:
        require(m["test_prefix"] + function + " ... ok" in texts[name], "named test " + function)
    for phase, stem, stages, count in [
        ("attemptreserved", "26-publication", publication_stages, 10),
        ("attemptconsumption", "27-consumption", consumption_stages, 16),
    ]:
        name = f"{stem}-repeat-{repeat}.log"
        require(records[name]["test_totals"] == [[1, 0, 0]], "process repeat")
        observed = signals(texts[name], phase)
        expected = {(stage, "SIGKILL", "9") for stage in stages} | {("AfterReturn", "control", "0")}
        require(len(observed) == count and set(observed) == expected, "signal matrix " + name)
        require("seed=0x074d stages=" + str(count - 1) in texts[name], "seed " + name)

for name, expected in [
    ("01-baseline.log", [39, 0, 0]),
    ("09-windows-regression.log", [204, 0, 0]),
    ("17-corrected-root-regression.log", [164, 0, 0]),
    ("29-linux-regression.log", [633, 0, 7]),
    ("30-linux-doctests.log", [43, 0, 0]),
    ("28-full-process-regression.log", [6, 0, 0]),
]:
    observed = [sum(row[i] for row in records[name]["test_totals"]) for i in range(3)]
    require(observed == expected, "regression " + name)
full = texts["28-full-process-regression.log"]
require(len(re.findall(r"P0 case=.* terminated=SIGKILL signal=9 ", full)) == 24, "old P0 signals")
require(len(re.findall(r"P0 case=.* terminated=control signal=0 ", full)) == 2, "old P0 controls")
require(len(signals(full, "attempt")) == 10, "old attempt phase")
require(len(signals(full, "attemptreserved")) == 10 and len(signals(full, "attemptconsumption")) == 16,
        "inherited phases regression")
require("contract input digest mismatch" in texts["36-qualification-refusal.log"], "qualification refusal")
require("ship admission is closed" in texts["37-ship-refusal.log"], "ship refusal")

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
    require(("tools/long-run-supervisor-074/src/diagnostic_attempt_publication_tests.rs", name)
            in registered, "W11 test " + name)
gates = tomllib.loads((ROOT / "docs/testing/gated-test-registry.toml").read_text())
for gate_id, full_name in zip(m["manual_gate_ids"], m["manual_test_full_names"], strict=True):
    require(gate_id in w11["gated_gate_ids"], "W11 gate")
    gate = next(item for item in gates["gate"] if item["id"] == gate_id)
    require(gate["tier"] == "manual" and not gate["ship_mandatory"]
            and gate["command"]["platform"] == "linux"
            and full_name in gate["command"]["args"], "manual registry")
print("attempt-transfer packet: OK; 3 x 24 owned SIGKILL + 2 controls; start/qualification closed")
