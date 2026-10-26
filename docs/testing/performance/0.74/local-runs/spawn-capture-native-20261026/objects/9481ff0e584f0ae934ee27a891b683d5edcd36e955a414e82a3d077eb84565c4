"""Pure single-target verdict join. Never dispatches or upgrades a package gate."""

from dataclasses import dataclass
import hashlib
import json
import re

from contract_frontier_audit import TARGET, case_catalog
from launch_preflight import ARGV, pinned
from owned_process_observer import Refusal
import review


def source_digest(source):
    return hashlib.sha256(json.dumps(source, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


@dataclass(frozen=True)
class ExpectedSource:
    """Caller-reserved prelaunch identity, never filled from the result being read."""
    head: str
    digest: str

    def __post_init__(self):
        if re.fullmatch(r"[0-9a-f]{40}", self.head) is None or re.fullmatch(r"[0-9a-f]{64}", self.digest) is None:
            raise Refusal("invalid reserved source identity")


def catalog_for(root):
    digest = "8b83fae322613d51aa5f3339b33f920df96670b934cf321c6e0573398ab0c54a"
    raw = pinned(root / "docs/testing/performance/0.74/local-runs/package-effect-syntax-v3-20261025/objects" / digest,
                 digest, 16 * 1024**2)
    names = frozenset(case_catalog(review.decode_json(raw))["cases"])
    if len(names) != 321:
        raise Refusal("independent 321-case catalog drift")
    return names


def join_target(intent, result, stdout, stderr, expected, catalog):
    """No attempt/lifetime claim: a live caller must separately bind its own files."""
    if intent["argv"] != ARGV or intent["bound_seconds"] != 780 or intent["retry"] is not False:
        raise Refusal("target command/window/retry drift")
    if intent["head"] != expected.head or source_digest(intent["source"]) != expected.digest:
        raise Refusal("target is not the caller-reserved source")
    if result["source_unchanged"] is not True or result["head_unchanged"] is not True or result["source_after"] != intent["source"]:
        raise Refusal("target source/HEAD changed")
    if len(catalog) != 321:
        raise Refusal("expected exactly 321 independent case names")
    parsed = review.parse_transcript(stdout, stderr, [TARGET], result["exit"], result["outer_refusal"], 780)
    relative_pass = parsed.pop("package_pass")
    if len(parsed["targets"]) != 1 or parsed["targets"][0]["declared"] != 321:
        raise Refusal("single target/case cardinality drift")
    observed = {row["name"] for row in parsed["targets"][0]["cases"]}
    if not observed <= catalog or (parsed["complete"] and observed != catalog):
        raise Refusal("actual qualified cases differ from independent catalog")
    if parsed["ignored"]:
        raise Refusal("ignored cases change the fixed scope")
    parsed.update(selected_target_pass=relative_pass, full_package_pass=False,
                  observation_complete_not_implied=True, live_attempt_binding_required=True,
                  admission=False)
    return parsed
