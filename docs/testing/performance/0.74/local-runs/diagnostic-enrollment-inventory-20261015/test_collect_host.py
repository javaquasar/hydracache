"""Local collector checks, never remote execution or production admission."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location("host_inventory", Path(__file__).with_name("collect_host.py"))
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)
supervisor_spec = importlib.util.spec_from_file_location("supervisor_inventory", Path(__file__).with_name("collect_supervisor.py"))
supervisor = importlib.util.module_from_spec(supervisor_spec)
supervisor_spec.loader.exec_module(supervisor)


class InventoryChecks(unittest.TestCase):
    def test_optional_absence_is_not_capability_failure_or_success(self):
        with patch.object(collector, "proc_text", side_effect=FileNotFoundError):
            self.assertEqual(collector.optional_kernel_text("fixed"), {"status": "absent-at-read"})

    def test_permission_is_unknown_not_absent(self):
        with patch.object(collector, "proc_text", side_effect=PermissionError):
            self.assertEqual(collector.optional_kernel_text("fixed"),
                             {"status": "unknown", "reason": "PermissionError"})

    def test_no_platform_or_privilege_fallback(self):
        with patch.object(collector.platform, "system", return_value="Windows"):
            with self.assertRaises(ValueError):
                collector.collect()

    def test_local_accounts_and_memberships_not_NSS_proof(self):
        result = collector.accounts(b"hydracache-perf:x:997:997::/:/bin/false\n"
                                    b"github-runner:x:1000:1000::/:/bin/false\n",
                                    b"hydracache-perf:x:997:\nhydracache-perf-client:x:998:github-runner\n")
        self.assertEqual(result["accounts"]["hydracache-perf"]["group_ids_including_primary"], [997])
        self.assertEqual(result["accounts"]["github-runner"]["group_ids_including_primary"], [998, 1000])
        self.assertIn("not-NSS", result["source"])
        self.assertEqual(result["named_groups"]["hydracache-perf-client"],
                         {"status": "available", "gid": 998, "audited_explicit_members": ["github-runner"]})

    def test_duplicate_or_missing_account_is_not_selected(self):
        result = collector.accounts(b"hydracache-perf:x:1:1::/:/bin/false\n" * 2, b"")
        self.assertEqual(result["accounts"]["hydracache-perf"]["status"], "absent-or-ambiguous")
        self.assertEqual(result["accounts"]["github-runner"]["status"], "absent-or-ambiguous")
        self.assertEqual(collector.accounts(None, b""), {"status": "unknown"})

    def test_only_fixed_public_files_not_credentials(self):
        self.assertEqual(len(collector.PUBLIC_FILES), 7)
        for name in collector.PUBLIC_FILES:
            self.assertTrue(name.startswith("/etc/"))
            self.assertFalse(any(word in name for word in ("shadow", "private", "seed", "token")))
        source = Path(collector.__file__).read_text(encoding="utf-8")
        self.assertNotIn("subprocess", source.split('"""', 2)[-1])
        self.assertNotIn("os.environ", source)

    @unittest.skipUnless(os.name == "posix", "real no-follow fixture needs Unix descriptor flags")
    def test_public_file_readonly_and_symlink_refusal(self):
        with tempfile.TemporaryDirectory() as root:
            leaf = Path(root) / "public"
            leaf.write_bytes(b"public bytes\n")
            before = leaf.stat()
            result, raw = collector.public_file(str(leaf))
            self.assertEqual(result["status"], "available")
            self.assertEqual(raw, b"public bytes\n")
            self.assertEqual(leaf.stat().st_mtime_ns, before.st_mtime_ns)
            link = Path(root) / "alias"
            link.symlink_to(leaf)
            self.assertEqual(collector.public_file(str(link))[0]["status"], "unknown")
            self.assertEqual(collector.public_file(str(Path(root) / "missing"))[0]["status"], "absent-at-read")

    @unittest.skipUnless(os.name == "posix", "real bounded-file fixture needs Unix descriptor flags")
    def test_over_budget_refuses_before_read(self):
        with tempfile.TemporaryDirectory() as root:
            leaf = Path(root) / "public"
            leaf.write_bytes(b"x" * 17)
            result, raw = collector.public_file(str(leaf), 16)
            self.assertEqual(result, {"status": "unknown", "reason": "ValueError"})
            self.assertIsNone(raw)


class SupervisorChecks(unittest.TestCase):
    def properties(self):
        return {key: {"MainPID": "8839", "ActiveState": "active",
                      "ControlGroup": supervisor.CGROUP}.get(key, "fixed")
                for key in supervisor.PROPERTIES}

    def test_only_fixed_readonly_systemd_properties(self):
        raw = "\n".join(f"{key}={value}" for key, value in self.properties().items()).encode()
        with patch.object(supervisor.subprocess, "run", return_value=SimpleNamespace(stdout=raw, stderr=b"")) as run:
            self.assertEqual(supervisor.unit(), self.properties())
        arguments, options = run.call_args
        self.assertEqual(arguments[0][:3], ["/usr/bin/systemctl", "show", supervisor.UNIT])
        self.assertEqual(options["timeout"], 5)
        self.assertEqual(set(options["env"]), {"PATH", "LANG", "LC_ALL"})
        self.assertFalse(any(word in raw for word in (b"Environment=", b"ExecStart=")))

    def test_wrong_cgroup_cannot_be_adopted(self):
        values = self.properties()
        values["ControlGroup"] = "/system.slice/replacement.service"
        raw = "\n".join(f"{key}={value}" for key, value in values.items()).encode()
        with patch.object(supervisor.subprocess, "run", return_value=SimpleNamespace(stdout=raw, stderr=b"")):
            with self.assertRaises(ValueError):
                supervisor.unit()

    def test_pid_generation_stat_projection(self):
        # Linux field 22 is offset 19 after the closing comm parenthesis.
        stat_raw = "8839 (name with ) bracket) " + " ".join(["S", *["0"] * 18, "12345", "0"])
        with patch.object(supervisor, "bounded", side_effect=[stat_raw, "0::" + supervisor.CGROUP]), \
                patch.object(supervisor.os, "stat", return_value=SimpleNamespace(st_dev=4, st_ino=7)):
            self.assertEqual(supervisor.observe(8839)["start_ticks"], 12345)

    def test_unit_or_process_drift_refuses(self):
        for kind in ("unit", "process"):
            first, last = self.properties(), self.properties()
            before = {"cgroup": "0::" + supervisor.CGROUP, "start_ticks": 1}
            after = dict(before)
            if kind == "unit":
                last["MainPID"] = "8840"
            else:
                after["start_ticks"] = 2
            with self.subTest(kind=kind), patch.object(supervisor, "unit", side_effect=[first, last]), \
                    patch.object(supervisor, "observe", side_effect=[before, after]), \
                    patch.object(supervisor.os, "geteuid", return_value=0, create=True):
                with self.assertRaises(ValueError):
                    supervisor.collect()

    def test_matching_inventory_never_grants_authority(self):
        observation = {"cgroup": "0::" + supervisor.CGROUP}
        with patch.object(supervisor, "unit", return_value=self.properties()), \
                patch.object(supervisor, "observe", return_value=observation), \
                patch.object(supervisor.os, "geteuid", return_value=0, create=True), \
                patch.object(supervisor, "bounded", return_value="domain"), \
                patch.object(supervisor.Path, "is_file", return_value=True):
            result = supervisor.collect()
        self.assertTrue(result["read_only"])
        self.assertFalse(result["host_admission"])
        self.assertFalse(result["service_mutation"])
        self.assertTrue(result["numeric_pid_observation_not_original_start_or_cgroup_authority"])


if __name__ == "__main__":
    unittest.main()
