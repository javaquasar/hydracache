"""Synthetic public-source syntax checks only; no network or selected issuer key."""
import base64
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from scripts.perf import diagnostic_oidc_source_074 as source


def raw(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def fixtures():
    n = bytes([183]) + bytes(254) + bytes([1])
    return ({"issuer": source.ISSUER, "jwks_uri": source.JWKS_URI,
             "id_token_signing_alg_values_supported": ["RS256"]},
            {"keys": [{"kid": "synthetic-only", "kty": "RSA", "alg": "RS256", "use": "sig",
                       "n": base64.urlsafe_b64encode(n).rstrip(b"=").decode(), "e": "AQAB"}]})


class OidcSourceTests(unittest.TestCase):
    def test_inspection_digests_metadata_and_closed_admission(self):
        discovery, jwks = fixtures()
        report = source.inspect_sources(raw(discovery), raw(jwks))
        self.assertEqual(report["keys"][0]["modulus_bits"], 2048)
        self.assertEqual(report["keys"][0]["kid"], "synthetic-only")
        self.assertTrue(report["offline_parser_only"])
        for flag in ("authenticated_acquisition", "key_selected", "trust_config_created",
                     "protected_job_token_verified", "admission_allowed"):
            self.assertIs(report[flag], False)
        self.assertNotIn("n", report["keys"][0])
        self.assertNotIn("e", report["keys"][0])

    def test_bad_issuer_endpoint_algorithm_and_roster_refuse(self):
        discovery, jwks = fixtures()
        for field, value in (("issuer", "https://untrusted.invalid"), ("jwks_uri", "http://untrusted.invalid"),
                             ("id_token_signing_alg_values_supported", ["HS256"])):
            with self.assertRaises(ValueError):
                source.inspect_sources(raw({**discovery, field: value}), raw(jwks))
        for value in ([], None, {}, {"keys": []}, {"keys": jwks["keys"] * 9},
                      {"keys": jwks["keys"], "active_kid": "incoming-token-selected"}, {"keys": jwks["keys"] * 2}):
            with self.assertRaises(ValueError):
                source.inspect_sources(raw(discovery), raw(value))

    def test_private_key_fields_key_purpose_and_integer_confusion_refuse(self):
        discovery, jwks = fixtures()
        key = jwks["keys"][0]
        for field, value in (("d", "private-material-not-real"), ("k", "secret-not-real"), ("jku", "untrusted"),
                             ("alg", "none"), ("kty", "oct"), ("use", "enc"), ("kid", ""), ("kid", "\0"),
                             ("e", "Aw"), ("n", key["n"] + "="), ("n", "AA"), ("n", "bad!"),
                             ("x5c", "not-an-array"), ("x5t", 1)):
            with self.assertRaises(ValueError):
                source.inspect_sources(raw(discovery), raw({"keys": [{**key, field: value}]}))

    def test_duplicate_nested_fields_budgets_and_malformed_json_refuse(self):
        discovery, jwks = fixtures()
        key = raw(jwks["keys"][0]).decode()
        bad = ('{"keys":[' + key[:-1] + ',"kid":"synthetic-only"}]}').encode()
        for invalid in (b"", b"x" * (source.MAX_BYTES + 1), b"{", b"{\"keys\":NaN}", bad):
            with self.assertRaises(ValueError):
                source.inspect_sources(raw(discovery), invalid)
        with self.assertRaises(ValueError):
            source.inspect_sources(b'{"issuer":"x","issuer":"y"}', raw(jwks))

    def test_bounded_file_and_cli_are_readonly(self):
        discovery, jwks = fixtures()
        script = Path(source.__file__).resolve()
        with tempfile.TemporaryDirectory() as temp:
            a, b = Path(temp) / "discovery", Path(temp) / "jwks"
            a.write_bytes(raw(discovery))
            b.write_bytes(raw(jwks))
            before = {p.name: p.read_bytes() for p in Path(temp).iterdir()}
            self.assertEqual(source.read_bounded(a), raw(discovery))
            result = subprocess.run([sys.executable, str(script), str(a), str(b)], capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(json.loads(result.stdout)["authenticated_acquisition"])
            self.assertEqual(before, {p.name: p.read_bytes() for p in Path(temp).iterdir()})
            b.write_bytes(b"bad")
            result = subprocess.run([sys.executable, str(script), str(a), str(b)], capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 2)
            self.assertNotIn(b"bad", result.stderr)
            for invalid in (b"", b"x" * (source.MAX_BYTES + 1)):
                b.write_bytes(invalid)
                with self.assertRaises(ValueError):
                    source.read_bounded(b)

    def test_seeded_public_integer_mutations_never_select_trust(self):
        discovery, jwks = fixtures()
        seed = 0x07562074
        for _ in range(32):
            seed = ((seed ^ (seed << 13)) & ((1 << 64) - 1))
            seed ^= seed >> 7
            seed = ((seed ^ (seed << 17)) & ((1 << 64) - 1))
            changed = json.loads(raw(jwks))
            encoded = changed["keys"][0]["n"]
            offset = seed % len(encoded)
            changed["keys"][0]["n"] = encoded[:offset] + "!" + encoded[offset + 1:]
            with self.assertRaises(ValueError):
                source.inspect_sources(raw(discovery), raw(changed))
        print("seed=0x07562074 offline_mutations=32 key_selected=false authenticated_acquisition=false")


if __name__ == "__main__":
    unittest.main()
