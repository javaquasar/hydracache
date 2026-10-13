"""Read-only symbol receipt for the normal non-test supervisor library."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

artifact = Path(sys.argv[1]).resolve(strict=True)
assert artifact.name == "libhydracache_long_run_supervisor_074.rlib"
assert artifact.parent == Path("/home/artur/.cache/hydracache-074-manager-local-target/debug")
result = subprocess.run(["nm", "-a", "--defined-only", str(artifact)], capture_output=True, check=True)
assert b"publication_checkpoint" not in result.stdout
assert b"diagnostic_authority_checkpoint" not in result.stdout
print(json.dumps({
    "artifact": str(artifact),
    "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
    "cfg_test_observer_symbols": "absent",
    "symbol_inspector": "nm --defined-only",
    "production_activation": False,
}, sort_keys=True))
