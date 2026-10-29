"""Public encoding tests and owned disposable Linux key-custody fixtures."""
import hashlib
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from scripts.perf import diagnostic_worker_key_074 as keytool


class PublicEncodingTests(unittest.TestCase):
    def test_exact_ed25519_public_der_not_private_or_other_algorithm(self):
        public = bytes(range(32))
        self.assertEqual(keytool.public_bytes(keytool.SPKI_PREFIX + public), public)
        for raw in (b"", public, keytool.SPKI_PREFIX + public + b"x",
                    bytes([keytool.SPKI_PREFIX[0] ^ 1]) + keytool.SPKI_PREFIX[1:] + public):
            with self.assertRaises(keytool.Refused):
                keytool.public_bytes(raw)

    def test_canonical_public_selection_and_fingerprint(self):
        public = bytes(range(32))
        report = keytool.selection(public, bytes(range(64)), 1000)
        self.assertEqual(report["public_key_hex"], public.hex())
        self.assertEqual(report["public_key_sha256"], hashlib.sha256(public).hexdigest())
        self.assertFalse(report["worker_policy_signed"])
        self.assertFalse(report["admission_allowed"])
        self.assertIsNone(report["policy_epoch"])
        self.assertIsNone(report["worker_policy_body_sha256"])
        raw = keytool.canonical(report)
        self.assertEqual(json.loads(raw), report)
        self.assertTrue(raw.endswith(b"\n"))
        self.assertFalse(raw.endswith(b"\n\n"))

    def test_selfcheck_domain_cannot_be_worker_policy_message(self):
        self.assertTrue(keytool.PROBE.startswith(b"hydracache-diagnostic-worker-key-selfcheck-074-v1\0"))
        self.assertFalse(keytool.PROBE.startswith(b"hydracache-diagnostic-worker-policy-074-v1\0"))

    def test_subprocess_errors_never_repeat_stderr_or_private_stdout(self):
        result = subprocess.CompletedProcess([], 1, b"PRIVATE sentinel", b"PRIVATE sentinel")
        with patch.object(keytool.subprocess, "run", return_value=result), self.assertRaises(keytool.Refused) as error:
            keytool.openssl(["unused"], ())
        self.assertNotIn("PRIVATE", str(error.exception))

    def test_openssl_environment_does_not_inherit_provider_or_loader_overrides(self):
        result = subprocess.CompletedProcess([], 0, b"public", b"")
        with patch.dict(os.environ, {"OPENSSL_CONF": "foreign", "LD_PRELOAD": "foreign"}), \
                patch.object(keytool.subprocess, "run", return_value=result) as call:
            self.assertEqual(keytool.openssl(["version"], ()), b"public")
        self.assertEqual(call.call_args.kwargs["env"], {"PATH": "/usr/bin:/bin", "LANG": "C"})

    def test_unsupported_platform_refuses_before_any_file_or_crypto_operation(self):
        with patch.object(keytool.sys, "platform", "win32"), self.assertRaises(keytool.Refused):
            keytool.inspect(Path("unused"))
        with patch.object(keytool.sys, "platform", "linux"), \
                patch.object(keytool.os, "geteuid", return_value=0, create=True), self.assertRaises(keytool.Refused):
            keytool.inspect(Path("unused"))

    def test_metadata_ignores_read_atime_but_keeps_nanosecond_mutation_fields(self):
        from types import SimpleNamespace
        fields = dict(st_dev=1, st_ino=2, st_mode=0o100600, st_uid=1000, st_gid=1000,
                      st_nlink=1, st_size=119, st_mtime_ns=123, st_ctime_ns=456, st_atime_ns=789)
        original = keytool.stable_metadata(SimpleNamespace(**fields))
        self.assertEqual(original, keytool.stable_metadata(SimpleNamespace(**{**fields, "st_atime_ns": 999})))
        for field in fields.keys() - {"st_atime_ns"}:
            self.assertNotEqual(original, keytool.stable_metadata(SimpleNamespace(**{**fields, field: fields[field] + 1})))


@unittest.skipUnless(sys.platform == "linux" and os.geteuid() != 0,
                     "Custody fixtures require nonroot Linux/OpenSSL; not Windows/root coverage")
class LinuxCustodyTests(unittest.TestCase):
    def setUp(self):
        import pwd
        parent = Path(pwd.getpwuid(os.geteuid()).pw_dir)
        self.temp = tempfile.TemporaryDirectory(prefix="worker-key-test-074-", dir=parent)
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.target = self.root / "issuer"

    def test_generation_roundtrip_and_exact_modes(self):
        report = keytool.generate(self.target)
        self.assertEqual(keytool.inspect(self.target), report)
        self.assertEqual(stat.S_IMODE(self.target.stat().st_mode), 0o700)
        self.assertEqual(stat.S_IMODE((self.target / keytool.PRIVATE).stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE((self.target / keytool.PUBLIC).stat().st_mode), 0o400)
        self.assertEqual(len(bytes.fromhex(report["selfcheck_signature_hex"])), 64)

    def test_repeated_generation_does_not_overwrite_or_repair(self):
        keytool.generate(self.target)
        before = (self.target / keytool.PRIVATE).stat()
        public = (self.target / keytool.PUBLIC).read_bytes()
        with self.assertRaises((keytool.Refused, FileExistsError)):
            keytool.generate(self.target)
        self.assertEqual((self.target / keytool.PRIVATE).stat(), before)
        self.assertEqual((self.target / keytool.PUBLIC).read_bytes(), public)

    def test_symlinked_parent_target_and_private_file_refuse(self):
        alias = self.root / "alias"
        alias.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises((keytool.Refused, OSError)):
            keytool.generate(alias / "new")
        self.target.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises((keytool.Refused, OSError)):
            keytool.generate(self.target)
        self.target.unlink()
        keytool.generate(self.target)
        private = self.target / keytool.PRIVATE
        backup = self.target / "fixture-backup.pem"
        private.rename(backup)
        private.symlink_to(backup)
        with self.assertRaises((keytool.Refused, OSError)):
            keytool.inspect(self.target)

    def test_unsafe_ancestry_and_git_checkout_refuse_before_creation(self):
        self.root.chmod(0o770)
        try:
            with self.assertRaises(keytool.Refused):
                keytool.generate(self.target)
            self.assertFalse(self.target.exists())
        finally:
            self.root.chmod(0o700)  # Only this test's own disposable fixture.
        (self.root / ".git").mkdir()
        with self.assertRaises(keytool.Refused):
            keytool.generate(self.target)
        self.assertFalse(self.target.exists())

    def test_private_and_public_modes_refuse_without_repair(self):
        keytool.generate(self.target)
        for name, bad, original in ((keytool.PRIVATE, 0o640, 0o600), (keytool.PUBLIC, 0o644, 0o400)):
            file = self.target / name
            file.chmod(bad)
            with self.assertRaises(keytool.Refused):
                keytool.inspect(self.target)
            self.assertEqual(stat.S_IMODE(file.stat().st_mode), bad)
            file.chmod(original)

    def test_hardlinked_key_and_partial_generation_refuse(self):
        keytool.generate(self.target)
        os.link(self.target / keytool.PRIVATE, self.target / "fixture-hardlink.pem")
        with self.assertRaises(keytool.Refused):
            keytool.inspect(self.target)
        (self.target / "fixture-hardlink.pem").unlink()
        (self.target / keytool.PUBLIC).unlink()
        with self.assertRaises((keytool.Refused, OSError)):
            keytool.inspect(self.target)
        with self.assertRaises((keytool.Refused, OSError)):
            keytool.generate(self.target)

    def test_corrupt_public_selection_refuses(self):
        keytool.generate(self.target)
        public = self.target / keytool.PUBLIC
        public.chmod(0o600)
        public.write_bytes(b"{}\n")
        public.chmod(0o400)
        with self.assertRaises(keytool.Refused):
            keytool.inspect(self.target)

    def test_replaced_private_key_cannot_match_retained_public_selection(self):
        keytool.generate(self.target)
        other = self.root / "other"
        keytool.generate(other)
        os.replace(other / keytool.PRIVATE, self.target / keytool.PRIVATE)
        with self.assertRaises(keytool.Refused):
            keytool.inspect(self.target)

    def test_concurrent_generation_has_exactly_one_creator(self):
        def attempt():
            try:
                return keytool.generate(self.target)
            except FileExistsError:
                return None
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda _: attempt(), range(2)))
        successful = [report for report in results if report is not None]
        self.assertEqual(len(successful), 1)
        self.assertEqual(keytool.inspect(self.target), successful[0])

    def test_partial_crypto_failure_retains_private_attempt_without_retry(self):
        with patch.object(keytool, "openssl", side_effect=keytool.Refused("fixture failure")), self.assertRaises(keytool.Refused):
            keytool.generate(self.target)
        self.assertTrue(self.target.is_dir())
        self.assertEqual((self.target / keytool.PRIVATE).stat().st_size, 0)
        self.assertFalse((self.target / keytool.PUBLIC).exists())
        with self.assertRaises(FileExistsError):
            keytool.generate(self.target)
        with self.assertRaises(keytool.Refused):
            keytool.inspect(self.target)

    def test_special_and_oversized_private_files_refuse_before_crypto(self):
        self.target.mkdir(mode=0o700)
        private = self.target / keytool.PRIVATE
        os.mkfifo(private, 0o600)
        with patch.object(keytool, "openssl") as crypto, self.assertRaises(keytool.Refused):
            keytool.inspect(self.target)
        crypto.assert_not_called()
        private.unlink()
        private.write_bytes(b"x" * 4097)
        private.chmod(0o600)
        with patch.object(keytool, "openssl") as crypto, self.assertRaises(keytool.Refused):
            keytool.inspect(self.target)
        crypto.assert_not_called()


if __name__ == "__main__":
    unittest.main()
