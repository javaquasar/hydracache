"""Exercise the advisory template against cargo-audit without network access."""

from datetime import date
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

from scripts.security.upstream_advisory_input import upstream_lock


ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = ROOT / "docs/security/advisory-templates/hydra-fork-vulnerability.md"
ADVISORY_ID = "RUSTSEC-2099-0001"
LOCK_PACKAGE = """version = 4

[[package]]
name = "hydra-raft"
version = "{version}"
source = "registry+https://github.com/rust-lang/crates.io-index"
"""


class ForkAdvisoryTest(unittest.TestCase):
    def test_reports_first_hydra_revision_and_accepts_the_fixed_revision(self):
        with tempfile.TemporaryDirectory(prefix="hydra-fork-advisory-") as scratch:
            root = Path(scratch)
            db = root / "advisory-db"
            advisories = db / "crates/hydra-raft"
            advisories.mkdir(parents=True)
            contents = TEMPLATE.read_text(encoding="utf-8")
            contents = contents.replace("RUSTSEC-0000-0000", ADVISORY_ID)
            contents = contents.replace("YYYY-MM-DD", date.today().isoformat())
            (advisories / f"{ADVISORY_ID}.md").write_text(contents, encoding="utf-8")

            for version, expected_exit in (("0.7.0-hydra.1", 1), ("0.7.0-hydra.2", 0)):
                with self.subTest(version=version):
                    lock = root / f"{version}.lock"
                    lock.write_text(LOCK_PACKAGE.format(version=version), encoding="utf-8")
                    result = subprocess.run(
                        [
                            "cargo", "audit", "--db", str(db), "--no-fetch", "--no-yanked",
                            "--json", "-f", str(lock),
                        ],
                        cwd=root,
                        env={**os.environ, "CARGO_NET_OFFLINE": "true"},
                        capture_output=True,
                        text=True,
                        check=False,
                        timeout=30,
                    )
                    self.assertEqual(result.returncode, expected_exit, result.stderr or result.stdout)
                    findings = json.loads(result.stdout)["vulnerabilities"]
                    self.assertEqual(findings["count"], expected_exit)
                    if expected_exit:
                        actual, = findings["list"]
                        self.assertEqual(actual["advisory"]["id"], ADVISORY_ID)
                        self.assertEqual(actual["package"]["version"], version)

    def test_weekly_projection_uses_the_upstream_package_and_base_version(self):
        lock = tomllib.loads(upstream_lock({
            "hydra-raft": "0.7.0-hydra.1",
            "hydra-sqlx-macros-core": "0.8.6-hydra.2",
        }))
        self.assertEqual(
            [(package["name"], package["version"]) for package in lock["package"]],
            [("raft", "0.7.0"), ("sqlx-macros-core", "0.8.6")],
        )

    def test_weekly_projection_rejects_invalid_inventory(self):
        for forks in ({}, {"raft": "0.7.0-hydra.1"}, {"hydra-raft": "0.7.0"}):
            with self.subTest(forks=forks), self.assertRaises(ValueError):
                upstream_lock(forks)
