"""Offline evidence consistency only. Never loads trust, fetches or admits a host."""
import base64
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate field")
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw, object_pairs_hook=no_duplicates,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite")))


def record(name):
    return decode((HERE / name).read_bytes())


def check_records(host, supervisor, before, after, github, oidc, proposal):
    require(proposal["schema_version"] == "diagnostic-enrollment-review-proposal-074-v1", "proposal schema")
    for name in ("operator_selection_approved", "host_admission", "trust_config_installed",
                 "policy_signed", "workflow_enabled", "server_execution_allowed",
                 "qualification_allowed", "ship_allowed"):
        require(proposal[name] is False, "proposal promoted")
    require(proposal["reviewed_source"] == "323c2509a7d20969b8e9216340675883afa77894", "baseline changed")
    for item in (host, supervisor):
        require(item["read_only"] is True and item["host_admission"] is False
                and item["service_mutation"] is False and item["workload_started"] is False,
                "host inventory promoted")
    require(host["namespace_initial_host_attested"] is False and host["policy_epoch_selected"] is False
            and host["policy_signed"] is False, "host assertion promoted")
    h, p = host["context_after"], proposal["host_proposal"]
    require(host["context_before"] == h, "host context drift")
    require(p["machine_id"] == host["machine_id"] and p["boot_id"] == h["boot_id"], "host substitution")
    require(p["observed_host_namespaces"] == h["namespaces"]["1"], "host namespace substitution")
    s = supervisor["process_after"]
    require(supervisor["process_before"] == s and supervisor["unit_before"] == supervisor["unit_after"], "supervisor drift")
    require(p["observed_supervisor_namespaces"] == s["namespaces"], "supervisor namespace substitution")
    require(h["namespaces"]["thread-self"]["mnt"]["inode"] != s["namespaces"]["mnt"]["inode"],
            "namespace counterexample removed")
    require(p["actual_receiver_namespace_selected"] is False and p["actual_receiver_mount_namespace"] is None,
            "SSH context substituted for receiver")
    for name in ("worker", "controller", "oidc"):
        require(host["public_files"][f"/etc/hydracache-perf/{name}-trust-074.json"]["status"] == "absent-at-read",
                "installed trust assertion changed")
    require(host["directories"]["/var/lib/hydracache-performance/worker-authority-074"]["status"] == "absent-at-read",
            "authority observation changed")
    worker = proposal["worker_proposal"]
    require(worker["observed_current_epoch"] is None and worker["worker_policy_body_sha256"] is None
            and worker["epoch_initialization_approved"] is False and type(worker["proposed_initial_epoch"]) is int
            and worker["proposed_initial_epoch"] == 1
            and worker["receiver_binding_required_before_body_signing"] is True, "epoch or body fabricated")
    account = host["account_inventory"]["accounts"]["hydracache-perf"]
    for key in ("uid", "gid"):
        require(worker[key] == account[key] == 986, "worker account changed")
    require(worker["supplementary_gids"] == account["explicit_supplementary_gids"] == [], "worker groups changed")
    for name in ("passwd", "group"):
        require(worker[f"{name}_sha256"] == host["public_files"][f"/etc/{name}"]["sha256"], "account hash mismatch")
    for snapshot in (before, after):
        require(snapshot["read_only"] is True and snapshot["admission_allowed"] is False
                and snapshot["service_operations_performed"] is False and snapshot["product_process_started"] is False
                and snapshot["qualification_started"] is False and snapshot["promotable"] is False,
                "preflight promoted")
        for stage in ("lifecycle_before", "lifecycle_after"):
            lifecycle = snapshot[stage]
            require(all(marker["status"] == "absent" for marker in lifecycle["markers"]), "campaign marker")
            require(lifecycle["processes"]["complete"] is True, "incomplete process enumeration")
            require(lifecycle["supervisor"]["status"] == "available" and
                    "MainPID=8839\nNRestarts=0\n" in lifecycle["supervisor"]["stdout"], "supervisor changed")
        provisioning = snapshot["provisioning"]
        require(provisioning == before["provisioning"], "installed receipt drift")
        require(provisioning["fields"]["source_commit"] == "543108f1ccd206ae670803c2617f07e7fa91ae62", "installed source changed")
        for binary in snapshot["installed_binary_hashes"].values():
            require(binary["status"] == "available" and binary["sha256"] == provisioning["fields"]["binary_sha256"], "installed binary mismatch")
    require(github["read_only"] is True and github["workflow_dispatched"] is False
            and github["environment_changed"] is False and github["token_requested"] is False, "GitHub mutation")
    first = {r["id"]: r for r in github["first"]}
    for entry in [*github["first"], *github["last"], *github["secret_metadata"]]:
        require(entry["method"] == "GET", "mutating GitHub method")
    for entry in github["secret_metadata"]:
        listing = decode(entry["output"])
        require(entry["exit_code"] == 0 and listing["total_count"] == len(listing["secrets"]), "secret metadata incomplete")
        require(all(set(item) == {"name", "created_at", "updated_at"} for item in listing["secrets"]),
                "unexpected secret material")
    for entry in github["last"]:
        require(entry["output"] == first[entry["id"]]["output"] and entry["exit_code"] == 0, "GitHub state drift")
    subject = decode(first["oidc_subject"]["output"])
    require(subject == {"use_default": True, "use_immutable_subject": False,
                        "sub_claim_prefix": "repo:javaquasar/hydracache"}, "subject configuration differs")
    for name in ("controller_environment", "controller_workflow", "oidc_workflow_source"):
        require(first[name]["exit_code"] == 1 and "HTTP 404" in first[name]["output"], "missing source assertion differs")
    for name in ("running", "queued"):
        require(decode(first[name]["output"])["total_count"] == 0, "active GitHub run")
    environments = decode(first["environments"]["output"])
    require(environments["total_count"] == len(environments["environments"]), "environment enumeration incomplete")
    require(not any(e["name"] == proposal["job_proposal"]["environment"] for e in environments["environments"]), "controller environment appeared")
    reference = next(e for e in environments["environments"] if e["name"] == "performance-reference-074")
    require(reference["protection_rules"] == [], "reference protection observation differs")
    job = proposal["job_proposal"]
    require(job["workflow_sha"] is None and job["signer_build_receipt_sha256"] is None
            and job["signer_runtime_root_sha256"] is None and job["signer_runtime_custody_selected"] is False
            and job["actual_protected_job_token_verified"] is False, "job custody fabricated")
    require(job["proposed_subject"] == "repo:javaquasar/hydracache:environment:performance-diagnostic-controller-074",
            "proposed subject changed")
    spec = importlib.util.spec_from_file_location("oidc_public_source", REPO / "scripts/perf/diagnostic_oidc_source_074.py")
    source = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(source)
    inspection = source.inspect_sources(base64.b64decode(oidc["discovery_base64"], validate=True),
                                        base64.b64decode(oidc["jwks_base64"], validate=True))
    require(inspection == oidc["inspection"], "OIDC public source mismatch")
    require(oidc["authenticated_transport_observed"] is True and
            all(oidc[name] is False for name in ("key_selected", "trust_config_created", "protected_job_token_verified", "admission_allowed")),
            "OIDC acquisition promoted")
    roster = proposal["oidc_proposal"]
    require(roster["proposed_members"] == inspection["keys"] and roster["jwks_sha256"] == inspection["jwks_sha256"], "roster substitution")
    require(roster["selection_approved"] is False and roster["automatic_refresh"] is False, "roster promoted")
    require(roster["proposed_snapshot_not_before"] == oidc["received_before_unix_ns"] // 1000000000
            and roster["proposed_snapshot_expires_at"] - roster["proposed_snapshot_not_before"] == 86400,
            "snapshot validity changed")


def verify():
    manifest = record("manifest.json")
    require(manifest["schema_version"] == "diagnostic-enrollment-inventory-packet-074-v1", "manifest schema")
    require(manifest["admission_allowed"] is False and manifest["operator_selection_approved"] is False, "manifest promoted")
    for name, digest in manifest["artifacts"].items():
        path = Path(name)
        require(not path.is_absolute() and len(path.parts) == 1 and path.name not in (".", ".."), "unsafe artifact name")
        target = HERE / path
        require(not target.is_symlink() and target.is_file(), "unsafe artifact")
        raw = target.read_bytes()
        require(len(raw) == digest["bytes"] and hashlib.sha256(raw).hexdigest() == digest["sha256"], "artifact mismatch")
    for name, digest in manifest["unchanged_inputs"].items():
        require(hashlib.sha256((REPO / name).read_bytes()).hexdigest() == digest, "frozen input differs")
    check_records(*(record(name) for name in ("host-inventory.json", "supervisor-inventory.json", "preflight-before.json",
                                             "preflight-after.json", "github-inventory.json", "oidc-acquisition.json", "proposal.json")))
    print("Enrollment inventory hashes and observations verified; selection and admission remain closed.")


if __name__ == "__main__":
    verify()
