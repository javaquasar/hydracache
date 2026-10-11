"""Read-only local readiness audit; never enrollment or release qualification."""
import hashlib
import json
import re
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
m = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
expected = set(["readiness-console-audit-fixed.json","readiness-console-audit.json","readiness-console-build.log","readiness-console-ci-fixed.log","readiness-console-ci.log","readiness-console-explain.log","readiness-console-local-sbom.cdx.json","readiness-console-nested-red.log","readiness-console-package-metadata.log","readiness-console-prod-audit.json","readiness-console-security-green.log","readiness-console-security-red.log","readiness-console-test-final.log","readiness-console-test.log","readiness-console-unit-reviewed.log","readiness-isolation.log","readiness-linux-rustdoc.log","readiness-msrv-install.log","readiness-msrv-linux.log","readiness-toolchains.log","workspace-verify-windows.log","readiness-docs-final.log","readiness-book-final.log","readiness-links-format-final.log"])
require(m["schema_version"] == "production-readiness-local-evidence-074-v1", "schema")
require(m["state"] == "retained-local-readiness-not-production", "state")
require(m["evidence_class"] == "working-tree-local-readiness-audit-not-release-qualification", "class")
require(set(m["captures"]) == expected, "declared captures")
require({p.name for p in packet.iterdir() if p.suffix in {".log", ".json"} and p.name != "manifest.json"} == expected, "actual captures")
for name, pin in m["captures"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")


def text(name):
    return (packet / name).read_text(encoding="utf-8-sig")


def document(name):
    return json.loads(text(name))


full = text("workspace-verify-windows.log")
rows = [(int(p), int(i)) for p, i in re.findall(
    r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", full)]
require((len(rows), sum(p for p, _ in rows), sum(i for _, i in rows)) == (532, 3982, 82), "harness counts")
require("test result: FAILED" not in full and full.rstrip().endswith("verify: all gates passed"), "terminal success")
require("management console: SKIP (npm not found)" in full, "explicit skip")
v = m["full_workspace_verify"]
require(v["exit"] == 0 and v["all_gates_passed"] is True, "native exit")
require(v["mutable_working_tree_run"] is True and v["counts_are_unique_tests"] is False, "not an exact immutable receipt")
require((v["harness_rows"], v["harness_passes"], v["harness_ignored"]) == (532, 3982, 82), "count claims")
for name in ["readiness-console-test.log", "readiness-console-test-final.log"]:
    log = text(name)
    for marker in ["Tests  16 passed (16)", "pass 6", "fail 0", "46 passed", "wrote 182 components"]:
        require(marker in log, f"console {name}/{marker}")
reviewed = text("readiness-console-unit-reviewed.log")
for marker in ["Tests  16 passed (16)", "pass 7", "fail 0", "vulnerable nested copies", "wrote 182 components"]:
    require(marker in reviewed, f"reviewed unit {marker}")
require("fail 2" in text("readiness-console-security-red.log"), "test-first lock red")
require("pass 2" in text("readiness-console-security-green.log"), "lock green")
require("fail 1" in text("readiness-console-nested-red.log") and "true !== false" in text("readiness-console-nested-red.log"), "nested false-positive canary")
before = document("readiness-console-audit.json")
require(before["metadata"]["vulnerabilities"]["high"] == 2, "original high findings")
require(set(before["vulnerabilities"]) == {"brace-expansion", "source-map-js"}, "original packages")
for name in ["readiness-console-audit-fixed.json", "readiness-console-prod-audit.json"]:
    audit = document(name)
    require(audit["metadata"]["vulnerabilities"]["total"] == 0 and audit["vulnerabilities"] == {}, f"audit {name}")
sbom = document("readiness-console-local-sbom.cdx.json")
require(sbom["bomFormat"] == "CycloneDX" and len(sbom["components"]) == 182, "local SBOM")
versions = {row["name"]: row["version"] for row in sbom["components"]}
require(versions["brace-expansion"] == "5.0.12" and versions["source-map-js"] == "1.2.2", "SBOM fixed versions")
for name in ["readiness-msrv-linux.log", "readiness-linux-rustdoc.log"]:
    log = text(name)
    require("Finished `dev` profile" in log and "error:" not in log, f"Linux {name}")
lock = json.loads((repository / "console/package-lock.json").read_text(encoding="utf-8"))
for name, version in [("brace-expansion", "5.0.12"), ("source-map-js", "1.2.2")]:
    require(lock["packages"]["node_modules/" + name]["version"] == version, f"current lock {name}")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(m["qualification_manifest_sha256"] == pin and m["qualification_inputs_unchanged"] == 8, "frozen pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "current frozen bytes")
require("All 8 actual contract_inputs: unchanged" in text("readiness-isolation.log"), "isolation")
require(m["authority_decision_pending"] is True, "operator choice pending")
boundaries = ["all_threads_proved","positive_fixed_host_enrollment","durable_issuer_epoch_or_refusal","production_output_preparer","authenticated_original_start","production_diagnostic_backend","server_inspected_or_mutated","product_workload_or_qualification_run","performance_claim_allowed","registered_exact_candidate_fast_receipt","final_release_supply_chain_receipts","ship_admission"]
require(set(m["proof_boundaries"]) == set(boundaries) and all(v is False for v in m["proof_boundaries"].values()), "closed proof boundaries")
require(m["console"]["sbom_is_final_release_receipt"] is False, "local SBOM not a release receipt")
for marker in ["doc-check: OK", "performance-contract-check 0.74: OK", "readiness: 21 exact captures"]:
    require(marker in text("readiness-docs-final.log"), f"docs checkpoint {marker}")
require("Checked 51 markdown files and 20 external includes" in text("readiness-book-final.log"), "book links")
require("HTML book written" in text("readiness-book-final.log"), "book build")
require("Updated documentation: 168 local links resolve" in text("readiness-links-format-final.log"), "updated local links")
require("Final scoped rustfmt 1.94.0 passed" in text("readiness-links-format-final.log"), "final format")
require(m["final_quality_checks"]["packet_capture_count_at_docs_checkpoint"] == 21, "docs capture chronology")
print("readiness: 24 exact captures; working-tree verify green, console skip independently covered; production closed")
