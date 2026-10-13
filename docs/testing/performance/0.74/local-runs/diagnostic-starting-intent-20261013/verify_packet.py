"""Read-only exact-source owned Starting intent evidence, never host execution."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
BASELINE = "444ab967da19907261ade8c2fbd1d33e821a3869"
PREREGISTRATION = "a4a6d6e76b5346238d1eee89d0919eb64e5d6657"
IMPLEMENTATION = "07008eb43eb6606c7726d7d27799e995d2cddf6b"
IMPLEMENTATION_TREE = "5565559900294a98e4ed20f19f7dfb0cbbf3f909"
PROOF_SOURCE = "099bb14da0d17069913287a528ca2efc5a05caee"
PROOF_TREE = "0d736e15f86ac2b443f96493ce4dc0b9f295dbe3"
CODE_FILES = [
  ".gitattributes",
  "crates/xtask/tests/performance_contract_074.rs",
  "docs/COMPAT.md",
  "docs/GATES.md",
  "docs/testing/gated-test-registry.toml",
  "docs/testing/performance/0.74/diagnostic-starting-intent-local-design.md",
  "docs/testing/performance/0.74/diagnostic-starting-intent-local-contract.toml",
  "tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs",
  "tools/long-run-supervisor-074/src/diagnostic_starting_intent_storage.rs",
  "tools/long-run-supervisor-074/src/diagnostic_starting_intent_tests.rs",
  "tools/long-run-supervisor-074/src/diagnostic_authority_intent.rs",
  "tools/long-run-supervisor-074/src/diagnostic_authority_intent_tests.rs",
  "tools/long-run-supervisor-074/src/diagnostic_initial_consumption_storage.rs",
  "tools/long-run-supervisor-074/src/diagnostic_reservation_publication_storage.rs",
  "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
  "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
  "tools/long-run-supervisor-074/src/diagnostic_process_crash_tests.rs",
  "tools/long-run-supervisor-074/src/diagnostic_transition_tests.rs"
]
TEST_PREFIX = "diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::tests::starting_intent_tests::"
MANIFEST_HASH = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
PHASES = {
    "reserved": ["BeforeCreate","StateCreated","PrefixWritten","BytesWritten","FileSynced","DirectorySynced","DocumentBound","Acknowledging","AfterReturn"],
    "consumption": ["BeforeRecord","RecordCreated","RecordPrefixWritten","RecordBytesWritten","RecordSealed","RecordSynced","BeforeCommit","CommitCreated","CommitPrefixWritten","CommitBytesWritten","CommitSealed","CommitSynced","DirectorySynced","Acknowledging","AfterReturn"],
    "attempt": ["BeforeCreate","Created","PrefixWritten","BytesWritten","Sealed","FileSynced","DirectorySynced","Acknowledging","AfterReturn"],
    "attemptreserved": ["BeforeCreate","StateCreated","PrefixWritten","BytesWritten","FileSynced","DirectorySynced","DocumentBound","Acknowledging","AfterReturn"],
    "attemptconsumption": ["BeforeRecord","RecordCreated","RecordPrefixWritten","RecordBytesWritten","RecordSealed","RecordSynced","BeforeCommit","CommitCreated","CommitPrefixWritten","CommitBytesWritten","CommitSealed","CommitSynced","DirectorySynced","Acknowledging","AfterReturn"],
    "starting": ["BeforeCreate","Created","PrefixWritten","BytesWritten","Sealed","FileSynced","DirectorySynced","Acknowledging","AfterReturn"],
}

def require(ok, message):
    if not ok:
        raise SystemExit("FAIL: " + message)

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def lf(raw):
    return raw.replace(b"\r\n", b"\n")

def git(*args):
    return subprocess.check_output(["git","-C",str(ROOT),*args])

def totals(text):
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text)
    require(bool(rows), "missing successful test result")
    return tuple(sum(int(row[i]) for row in rows) for i in (0,1))

def crash(text, phases):
    rows = re.findall(r"P[01] case=(\w+):(\w+) terminated=(SIGKILL|control) signal=(\d+) fresh_processes=2 retained=unchanged original_lock=released helper=terminal", text)
    expected = {(phase, stage, "SIGKILL", "9") for phase in phases for stage in PHASES[phase]}
    expected.update((phase,"AfterReturn","control","0") for phase in phases)
    require(set(rows) == expected and len(rows) == len(expected), "complete exact crash/control set")
    require("seed=0x074d" in text, "process seed")
    return len(rows) * 2

m = json.loads((PACKET/"manifest.json").read_bytes())
require(m["schema_version"] == 1 and m["state"] == "local-owned-first-starting-intent-only", "scope")
for key, value in (("baseline_source",BASELINE),("preregistration_source",PREREGISTRATION),
                   ("implementation_source",IMPLEMENTATION),("implementation_tree",IMPLEMENTATION_TREE),
                   ("proof_source",PROOF_SOURCE),("proof_tree",PROOF_TREE)):
    require(m[key] == value, key)
require(git("rev-parse",IMPLEMENTATION+"^{tree}").decode().strip() == IMPLEMENTATION_TREE, "implementation tree")
require(git("rev-parse",PROOF_SOURCE+"^{tree}").decode().strip() == PROOF_TREE, "proof tree")
for a,b in ((BASELINE,PREREGISTRATION),(PREREGISTRATION,IMPLEMENTATION),(IMPLEMENTATION,PROOF_SOURCE),(PROOF_SOURCE,"HEAD")):
    git("merge-base","--is-ancestor",a,b)
require(git("diff","--name-only",IMPLEMENTATION,PROOF_SOURCE).decode().splitlines() ==
        ["tools/long-run-supervisor-074/src/diagnostic_transition_tests.rs"], "test-only fence correction")
require(set(m["proof_source_sha256"]) == set(CODE_FILES), "source inventory")
for path, expected in m["proof_source_sha256"].items():
    require(digest(lf(git("show",PROOF_SOURCE+":"+path))) == expected, "committed source "+path)
    require(digest(lf((ROOT/path).read_bytes())) == expected, "current source "+path)
for path, expected in m["unchanged_sources"].items():
    require(digest(lf(git("show",BASELINE+":"+path))) == expected, "original source "+path)
    require(digest(lf((ROOT/path).read_bytes())) == expected, "unchanged source "+path)
contract = tomllib.loads((ROOT/"docs/testing/performance/0.74/diagnostic-starting-intent-local-contract.toml").read_text())
require(contract == m["contract_pins"], "complete contract")
for key in ("starting_to_running_implemented","later_fixed_cells_implemented","full_fixed_lifecycle_implemented",
            "completion_or_cgroup_cleanup_authenticated","authenticated_peer_or_clock","global_nonce_protection",
            "machine_power_loss_proved","host_reboot_proved","positive_host_enrollment_proved",
            "server_entry_wired","worker_execution_allowed","server_operation_performed",
            "qualification_allowed","ship_allowed","performance_claim_allowed"):
    require(contract[key] is False, "closed "+key)
require(contract["max_record_bytes"] == 4096 and contract["record_schema"] == "diagnostic-starting-intent-074-v1"
        and contract["record_name"] == "diagnostic-starting-intent-074.json", "record limits")
require(contract["new_durable_record"] is True and contract["initial_active_state_immutable_anchor"] is True, "storage scope")
require(contract["functional_test_seed"] == "0x074b" and contract["process_test_seed"] == "0x074d"
        and contract["targeted_repetitions"] == 3, "fixed seeds and repeats")
require(digest(lf(Path(__file__).read_bytes())) == m["verifier_sha256"], "verifier")
files = m["files"]
require(len(files) == m["raw_log_count"] == 53 and len({f["path"] for f in files}) == len(files), "unique capture count")
require({p.name for p in PACKET.glob("*.log")} == {f["path"] for f in files}, "exact capture inventory")
logs = {}
negative = set(m["retained_development_failures"])
for f in files:
    require(Path(f["path"]).name == f["path"] and f["platform"] in ("linux","windows") and f["command"], "capture metadata")
    if f["id"] in negative:
        require(f["intended_exit_code"] == 0 and f["observed_exit_code"] == 101, "retained unexpected failure "+f["id"])
    else:
        require(f["observed_exit_code"] == f["intended_exit_code"], "exit "+f["id"])
    raw = (PACKET/f["path"]).read_bytes()
    require(len(raw) == f["bytes"] and digest(raw) == f["sha256"], "raw hash "+f["id"])
    logs[f["id"]] = raw.decode("utf-8",errors="replace")
require(totals(logs["baseline"]) == (26,1), "baseline selected filters")
require("E0432" in logs["red-missing-entry"] and "starting_intent" in logs["red-missing-entry"], "missing-entry red")
require("E0277" in logs["first-healthy"], "generic I/O conversion negative")
for name in ("expanded-functional","linux-clippy-step"):
    require("PublicationStageAuthority" in logs[name] and "E0433" in logs[name], "missing test alias")
require("diagnostic_starting_intent_retains_original_anchor_without_execution ... FAILED" in logs["root-step"]
        and "sync_all().map_err" in logs["root-step"], "formatter-sensitive source guard negative")
for name in ("starting-process-repeat-1","starting-process-repeat-3"):
    require("owned child did not reach its logical checkpoint: Timeout" in logs[name], "retained readiness negative")
require("transition_publisher_refuses_invalid_candidate_before_any_write ... FAILED" in logs["linux-regression"]
        and "Err" in logs["linux-regression"] and "Busy" in logs["linux-regression"], "retained pre-correction fixture failure")
require(totals(logs["corrected-functional"]) == (8,0), "initial eight cases")
require(totals(logs["functional-expanded-corrected"]) == (14,0), "expanded fourteen cases")
require(totals(logs["functional-pre-freeze"]) == (16,0), "sixteen cases before freeze")
for name in ("fixture-correction-focus","fixture-correction-windows"):
    require(totals(logs[name]) == (8,0), "fence fixture correction")
for n in range(1,4):
    text = logs["final-functional-"+str(n)]
    require(totals(text) == (16,0), "final functional repeat")
    for name in m["required_test_names"]:
        require("test "+TEST_PREFIX+name+" ... ok" in text, "named final case "+name)
    require("Starting graph seed=0x074b legal=128 rejected=128 execution=closed" in text, "seeded graph")
    text = logs["final-starting-process-"+str(n)]
    require(totals(text) == (1,0) and crash(text,["starting"]) == 20, "final Starting process repeat")
require(totals(logs["final-all-process-regression"]) == (7,0)
        and crash(logs["final-all-process-regression"],PHASES) == 144, "final prior-phase crash regression")
require(totals(logs["all-process-regression"]) == (7,0)
        and crash(logs["all-process-regression"],PHASES) == 144, "initial full crash regression")
require(totals(logs["final-linux-regression"]) == (660,8), "final Linux functional regression")
require(totals(logs["windows-regression"]) == (204,0), "Windows full regression")
require(totals(logs["final-windows-regression"]) == (204,0), "final Windows full regression")
require(totals(logs["root-corrected"]) == (166,0), "root contracts")
require(totals(logs["linux-rustdoc"]) == (48,0), "Linux no-conversion doctests")
require("starting_intent (line" in logs["linux-rustdoc"], "new compile-fail module")
require("ship admission is closed" in logs["ship-refusal"], "ship refusal")
require("contract input digest mismatch" in logs["qualification-refusal"], "qualification refusal")
require(not Path(m["qualification_output_must_not_exist"]).exists(), "no qualification artifact")
qpath = ROOT/"docs/testing/performance/0.74/qualification-manifest.toml"
require(digest(qpath.read_bytes()) == MANIFEST_HASH == m["qualification_manifest_sha256"], "raw qualification manifest")
q = tomllib.loads(qpath.read_text())
require(len(q["contract_inputs"]) == 8 and q["candidate_source_sha"] == "UNRESOLVED" and not q["expensive_runs_enabled"], "closed qualification")
require(set(m["frozen_inputs_sha256"]) == {f["path"] for f in q["contract_inputs"]}, "frozen inputs")
for path, expected in m["frozen_inputs_sha256"].items():
    require(digest(lf(git("show",BASELINE+":"+path))) == expected, "frozen baseline "+path)
    require(digest(lf((ROOT/path).read_bytes())) == expected, "frozen current "+path)
plan = next(f for f in q["contract_inputs"] if f["path"].startswith("docs/plans/"))
require(plan["sha256"] == "6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d"
        and m["frozen_inputs_sha256"][plan["path"]] == "cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828", "unchanged inherited mismatch")
historical = "docs/testing/performance/0.74/local-runs/diagnostic-signed-history-20261013"
old = json.loads((ROOT/historical/"manifest.json").read_bytes())
for name in ["manifest.json"]+[f["path"] for f in old["files"]]+[f["path"] for f in old["supplemental_checks"]]:
    require((ROOT/historical/name).read_bytes() == git("show",BASELINE+":"+historical+"/"+name), "historical raw "+name)
registry_path = "docs/testing/release-evidence/0.74.toml"
expected_registry = tomllib.loads(git("show",PROOF_SOURCE+":"+registry_path).decode())
w11 = next(w for w in expected_registry["work_item"] if w["id"] == "W11")
w11["required_artifacts"].extend("docs/testing/performance/0.74/local-runs/diagnostic-starting-intent-20261013/"+name for name in ("README.md","manifest.json","verify_packet.py"))
require(tomllib.loads((ROOT/registry_path).read_text()) == expected_registry, "scoped W11 evidence-only addition")
gates = tomllib.loads((ROOT/"docs/testing/gated-test-registry.toml").read_text())["gate"]
g = next(g for g in gates if g["id"] == "tool.hydracache-long-run-supervisor-074.diagnostic-process-crash-starting-all-stages-and-post-return")
require(g["tier"] == "manual" and g["ship_mandatory"] is False and g["timeout_seconds"] == 600
        and g["command"]["platform"] == "linux"
        and "diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::mount_binding::task_binding::authority_binding::intent_binding::tests::process_crash_tests::diagnostic_process_crash_starting_all_stages_and_post_return" in g["command"]["args"], "registered bounded manual proof")
print("PASS: owned initial Starting intent; three final-source repeats; 27 SIGKILL and three controls; no execution or qualification")
