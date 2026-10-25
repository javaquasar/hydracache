"""Pinned local launch receipt reader. CLI never dispatches a target."""

import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import sys

from owned_process_observer import Refusal
from process_lineage import normalize


RECEIPT = "docs/testing/performance/0.74/contract-observer-launch-receipt-20261025.json"
RECEIPT_SHA256 = "0f364991c37248f7f7d73d271932b3aaa77250d308df4a6314b0eb09905d65b5"
ARGV = ["cargo", "+1.94.0", "test", "--offline", "--locked", "-p", "xtask",
        "--test", "performance_contract_074", "--", "--test-threads=4"]
BUILD_ARGV = ARGV[:-2] + ["--no-run", "--message-format=json"]


def require(condition, reason):
    if not condition:
        raise Refusal(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw, object_pairs_hook=unique_pairs)


def pinned(path, digest, limit=128 * 1024**2):
    require(path.is_file() and path.stat().st_size <= limit, "missing or oversized pinned file: " + str(path))
    raw = path.read_bytes()
    require(sha(raw) == digest, "pinned bytes drift: " + str(path))
    return raw


def source_inputs(source):
    return {name: digest for name, digest in source.items()
            if name.startswith(("crates/", ".cargo/")) or name in ("Cargo.toml", "Cargo.lock", "deny.toml")
            or (name.startswith("tools/") and name.endswith((".rs", ".toml")))}


def source_path(root, name):
    path = PurePosixPath(name)
    require(not path.is_absolute() and ".." not in path.parts and "\\" not in name and ":" not in name,
            "unsafe source inventory path")
    return root / path


def check_build(receipt, before, after, raw_transcript):
    require(before["argv"] == BUILD_ARGV and before["head"] == receipt["build_source_head"], "build command/source-head mismatch")
    require(after["exit"] == 0 and after["outer_refusal"] is None and after["source_unchanged"] is True
            and after["head_unchanged"] is True and before["source"] == after["source_after"], "build did not preserve source/HEAD")
    inputs = source_inputs(before["source"])
    require(len(inputs) == receipt["build_source_count"]
            and sha(json.dumps(inputs, sort_keys=True, separators=(",", ":")).encode()) == receipt["build_source_fingerprint"],
            "declared build-input inventory drift")
    rows = [decode(line) for line in raw_transcript.splitlines() if line.strip()]
    finished = [row for row in rows if row.get("reason") == "build-finished"]
    require(len(finished) == 1 and finished[0].get("success") is True and rows[-1] == finished[0], "incomplete no-run build")
    artifacts = [row for row in rows if row.get("reason") == "compiler-artifact"
                 and row.get("target", {}).get("name") == "performance_contract_074"]
    require(len(artifacts) == 1, "ambiguous/missing test artifact")
    artifact = artifacts[0]
    require(artifact["target"]["kind"] == ["test"] and artifact["features"] == [], "test kind/features drift")
    require(artifact["profile"] == dict(opt_level="0", debuginfo=2, debug_assertions=True,
                                      overflow_checks=True, test=True), "test profile drift")
    require(normalize(artifact["executable"]) == normalize(receipt["images"]["test"]["path"]), "test executable path drift")
    require(normalize(artifact["manifest_path"]) == normalize(receipt["cwd"] + r"\crates\xtask\Cargo.toml")
            and normalize(artifact["target"]["src_path"]) == normalize(receipt["cwd"] + r"\crates\xtask\tests\performance_contract_074.rs"),
            "test artifact source/manifest drift")
    return inputs


def evaluate(root):
    document = decode(pinned(root / RECEIPT, RECEIPT_SHA256, 65536))
    require(document["schema"] == "hc074-local-contract-observer-launch-v1" and document["dispatch_authorized"] is False,
            "receipt is not preflight-only")
    require(document["argv"] == ARGV and document["wrapper_seconds"] == 780, "original command/window drift")
    require(normalize(str(root.resolve())) == normalize(document["cwd"]), "original CWD drift")
    require(normalize(sys.executable) == normalize(document["images"]["python"]["path"]), "Python resolution drift")
    resolved_cargo = shutil.which("cargo")
    require(resolved_cargo is not None and normalize(resolved_cargo) == normalize(document["images"]["shim"]["path"]), "Cargo PATH resolution drift")
    for name, digest in document["wrappers"].items():
        pinned(source_path(root, name), digest)
    for image in document["images"].values():
        pinned(Path(image["path"]), image["sha256"])
    build = {name: pinned(root / "docs/testing/performance/0.74/local-runs/launch-build-20261025/objects" / digest, digest, 16 * 1024**2)
             for name, digest in document["build_objects"].items()}
    inputs = check_build(document, decode(build["intent.json"]), decode(build["result.json"]), build["stdout.log"])
    for name, digest in inputs.items():
        pinned(source_path(root, name), digest)
    # Reuse the independently pinned source/effect and environment gates. Do not
    # change their pins or infer admission from newly discovered process paths.
    sys.path.insert(0, str(root / "tools/historical-blob-batch-074"))
    import package_effects_review_v3 as gate
    ast_digest = "8b83fae322613d51aa5f3339b33f920df96670b934cf321c6e0573398ab0c54a"
    ast = decode(pinned(root / "docs/testing/performance/0.74/local-runs/package-effect-syntax-v3-20261025/objects" / ast_digest, ast_digest, 16 * 1024**2))
    reviewed = gate.source_review(root, ast)
    environment = gate.previous.environment_preflight(root)
    policies = [[document["images"][role]["path"] for role in shape] for shape in document["policy_roles"]]
    return dict(schema=1, receipt_sha256=RECEIPT_SHA256, planned_argv=ARGV, planned_policies=policies,
                build_source_inputs=len(inputs), reviewed_rust_sources=len(reviewed["sources"]),
                environment=environment, preflight_pass=True, actual_cargo_shape_observed=False,
                dispatch_authorized=False, tests_executed=False, mapped_image_attested=False, admission=False)


def main(output):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / "intent.json").write_text(json.dumps(dict(receipt_sha256=RECEIPT_SHA256,
                                                    dispatch=False, retry=False)), encoding="utf-8")
    try:
        result = evaluate(Path(__file__).resolve().parents[2])
    except Exception as error:
        result = dict(preflight_pass=False, first_refusal=f"{type(error).__name__}: {error}", dispatch_authorized=False, admission=False)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result["preflight_pass"]


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "preflight":
        raise SystemExit("usage: launch_preflight.py preflight NEW_OUTPUT_DIR (no dispatch mode)")
    raise SystemExit(0 if main(sys.argv[2]) else 1)
