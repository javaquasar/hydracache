"""Offline public discovery/JWKS syntax inspection; no network, token or trust writer."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import stat

ISSUER = "https://token.actions.githubusercontent.com"
JWKS_URI = ISSUER + "/.well-known/jwks"
MAX_BYTES = 65536


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate source field")
        result[key] = value
    return result


def document(raw):
    if not 0 < len(raw) <= MAX_BYTES:
        raise ValueError("source budget refused")
    return json.loads(raw, object_pairs_hook=unique,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite source")))


def integer(text):
    if not isinstance(text, str) or not 0 < len(text) <= 684 or "=" in text:
        raise ValueError("public integer refused")
    raw = base64.b64decode(text + "=" * (-len(text) % 4), altchars=b"-_", validate=True)
    if base64.urlsafe_b64encode(raw).rstrip(b"=").decode() != text or not raw or raw[0] == 0:
        raise ValueError("noncanonical public integer")
    return raw


def inspect_sources(discovery_raw, jwks_raw):
    discovery, jwks = document(discovery_raw), document(jwks_raw)
    if not isinstance(discovery, dict) or discovery.get("issuer") != ISSUER or discovery.get("jwks_uri") != JWKS_URI:
        raise ValueError("fixed issuer/endpoint refused")
    if discovery.get("id_token_signing_alg_values_supported") != ["RS256"]:
        raise ValueError("issuer algorithm profile refused")
    if not isinstance(jwks, dict) or set(jwks) != {"keys"} or not isinstance(jwks["keys"], list) or not 0 < len(jwks["keys"]) <= 8:
        raise ValueError("public key roster refused")
    results, kids = [], set()
    for key in jwks["keys"]:
        required = {"kid", "kty", "alg", "use", "n", "e"}
        optional = {"x5c", "x5t", "x5t#S256"}
        if not isinstance(key, dict) or not required <= set(key) or set(key) - required - optional:
            raise ValueError("public key fields refused")
        kid = key["kid"]
        if not isinstance(kid, str) or not 0 < len(kid) <= 128 or any(ord(c) < 32 or ord(c) == 127 for c in kid) or kid in kids:
            raise ValueError("public key identifier refused")
        if (key["kty"], key["alg"], key["use"]) != ("RSA", "RS256", "sig"):
            raise ValueError("public key purpose refused")
        n, e = integer(key["n"]), integer(key["e"])
        if not 256 <= len(n) <= 512 or n[0] & 128 == 0 or n[-1] & 1 == 0 or e != b"\x01\x00\x01":
            raise ValueError("public RSA bounds refused")
        if "x5c" in key and (not isinstance(key["x5c"], list) or not 0 < len(key["x5c"]) <= 4 or
                             any(not isinstance(c, str) or not 0 < len(c) <= 4096 for c in key["x5c"])):
            raise ValueError("certificate metadata refused")
        for field in ("x5t", "x5t#S256"):
            if field in key and (not isinstance(key[field], str) or not 0 < len(key[field]) <= 128):
                raise ValueError("thumbprint metadata refused")
        kids.add(kid)
        projection = json.dumps({"n": key["n"], "e": key["e"]}, sort_keys=True, separators=(",", ":")).encode()
        results.append({"kid": kid, "modulus_bits": len(n) * 8, "public_key_projection_sha256": hashlib.sha256(projection).hexdigest()})
    return {"schema_version": "diagnostic-oidc-source-inspection-074-v1", "issuer": ISSUER, "jwks_uri": JWKS_URI,
            "discovery_sha256": hashlib.sha256(discovery_raw).hexdigest(), "jwks_sha256": hashlib.sha256(jwks_raw).hexdigest(),
            "keys": sorted(results, key=lambda k: k["kid"]), "offline_parser_only": True,
            "authenticated_acquisition": False, "key_selected": False, "trust_config_created": False,
            "protected_job_token_verified": False, "admission_allowed": False}


def read_bounded(path):
    if path.is_symlink():
        raise ValueError("source link refused")
    before = path.stat()
    if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1 or not 0 < before.st_size <= MAX_BYTES:
        raise ValueError("source file refused")
    with path.open("rb") as handle:
        raw = handle.read(MAX_BYTES + 1)
    after = path.stat()
    stamp = lambda s: (s.st_dev, s.st_ino, s.st_mode, s.st_uid, s.st_gid,
                       s.st_nlink, s.st_size, s.st_mtime_ns, s.st_ctime_ns)
    if stamp(before) != stamp(after) or path.is_symlink() or len(raw) != before.st_size:
        raise ValueError("source file changed")
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("discovery", type=Path)
    parser.add_argument("jwks", type=Path)
    args = parser.parse_args()
    try:
        result = inspect_sources(read_bounded(args.discovery), read_bounded(args.jwks))
    except (ValueError, OSError, RecursionError, TypeError, KeyError):
        parser.exit(2, "Public OIDC source inspection refused; no trust or admission created.\n")
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
