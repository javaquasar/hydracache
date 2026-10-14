"""Read-only kernel-input evidence check; never installed-host admission."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

P = Path(__file__).resolve().parent
R = P.parents[5]
M = json.loads((P / "manifest.json").read_text())


def lf(raw):
    return raw.replace(b"\r\n", b"\n")


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def git(commit, path):
    return lf(subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=R))


def passed(log):
    return sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", log)))


assert M["baseline_source"] == "5483ae1c0d37dc925c464662c1f88c8db83e150a"
assert M["implementation_source"] == "f2dbbe64c0fbf0a0eeca3bf6e8e2177188bc7f67"
for commit in (M["baseline_source"], M["preregistration_source"], M["implementation_source"]):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    subprocess.check_call(["git", "cat-file", "-e", commit + "^{commit}"], cwd=R)
assert sha(lf((P / "verify_packet.py").read_bytes())) == M["verifier_sha256_lf"]
for entry in M["source_files"]:
    raw = git(M["implementation_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((R / entry["path"]).read_bytes()) == raw
for entry in M["frozen_files"]:
    raw = git(M["baseline_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((R / entry["path"]).read_bytes()) == raw
assert len(M["source_files"]) == 10 and len(M["frozen_files"]) == 8
changed = set(subprocess.check_output([
    "git", "diff", "--name-only", M["baseline_source"], M["implementation_source"]
], cwd=R, text=True).splitlines())
designs = {"docs/testing/performance/0.74/diagnostic-kernel-admission-design.md",
           "docs/testing/performance/0.74/diagnostic-kernel-admission-contract.toml"}
assert changed == {entry["path"] for entry in M["source_files"]} | designs
for path in designs:
    raw = git(M["preregistration_source"], path)
    assert git(M["implementation_source"], path) == raw == lf((R / path).read_bytes())
lib = "tools/long-run-supervisor-074/src/lib.rs"
declaration = b'#[cfg(target_os = "linux")]\npub mod diagnostic_kernel_admission;\n'
raw = git(M["implementation_source"], lib)
assert raw.count(declaration) == 1
assert raw.replace(declaration, b"") == git(M["baseline_source"], lib)
unix = "tools/long-run-supervisor-074/src/unix_transport.rs"
raw = git(M["implementation_source"], unix).decode()
method = raw.split("impl SeqpacketConnection {\n", 1)[1].split("    pub fn connect", 1)[0]
assert "into_diagnostic_kernel_receiver" in method
assert raw.replace(method, "", 1).encode() == git(M["baseline_source"], unix)
for path in ("crates", "scripts", "Cargo.toml", "Cargo.lock"):
    assert not subprocess.check_output([
        "git", "diff", "--name-only", M["baseline_source"], M["implementation_source"], "--", path
    ], cwd=R)
assert sha((R / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == M["qualification_sha256_raw"]
contract = tomllib.loads((R / "docs/testing/performance/0.74/diagnostic-kernel-admission-contract.toml").read_text())
assert contract["packet_bytes"] == 16384 and contract["ancillary_bytes"] == 1088
assert contract["lease_clock"] == "CLOCK_BOOTTIME" and contract["freshness_clock"] == "CLOCK_REALTIME"
for flag in ("production_issuer_enrolled", "production_host_enrolled", "original_start_implemented",
             "completion_authority_exposed", "server_operation_allowed", "qualification_allowed", "ship_allowed"):
    assert contract[flag] is False
logs = {}
for entry in M["captures"]:
    assert entry["path"] == entry["id"] + ".log" and entry["id"] not in logs
    raw = (P / entry["path"]).read_bytes()
    assert sha(raw) == entry["sha256"] and len(raw) == entry["bytes"]
    assert entry["command"]
    if entry["id"] in ("first-tests", "review-tests-clippy"):
        assert entry["expected_exit"] == 0 and entry["observed_exit"] == 101
    else:
        assert entry["observed_exit"] == entry["expected_exit"]
    logs[entry["id"]] = raw.decode("utf-8")
assert len(logs) == M["capture_count"]
assert passed(logs["baseline"]) == 30
assert "E0599" in logs["red"] and "into_diagnostic_kernel_receiver" in logs["red"]
assert "compiler unexpectedly panicked" in logs["first-tests"]
assert "manual_is_multiple_of" in logs["review-tests-clippy"]
for repeat in (1, 2, 3):
    assert passed(logs[f"reviewed-fast-{repeat}"]) == 14
    assert "1 ignored" in logs[f"reviewed-fast-{repeat}"]
    manual = logs[f"reviewed-manual-{repeat}"]
    assert passed(manual) == 1
    assert "connection_peer=parent inherited_sender=child per_message_pidfd=original actual_sigkill=1 exited_sender=refused parent_sender=still_live production_enrollment=false" in manual
assert passed(logs["transport-regression"]) == 31
assert passed(logs["collector-regression"]) == 20
assert passed(logs["doctests"]) == 58
assert passed(logs["root-contracts"]) == 143
assert passed(logs["root-artifacts-final"]) == 13
assert "gated-test-check: OK" in logs["gated-check"]
assert "doc-check: OK" in logs["doc-check"]
assert "non-promotable" in logs["performance-contract"]
assert "ship admission is closed" in logs["ship-refusal"]
assert M["server_operation"] is False and M["production_enrollment"] is False
print(f"kernel-input packet verified: {len(logs)} captures; production trust/start/completion remain closed")
