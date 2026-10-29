"""Fixed public HTTPS acquisition tool; unsigned observation, never trust enrollment."""
import base64
import hashlib
import http.client
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys
import time

if __package__:
    from . import diagnostic_oidc_source_074 as source
else:
    import diagnostic_oidc_source_074 as source

HOST = "token.actions.githubusercontent.com"
PATHS = ("/.well-known/openid-configuration", "/.well-known/jwks")
SCHEMA = "diagnostic-oidc-public-acquisition-074-v1"
MAX_REPORT_BYTES = 196608
WORKER_TIMEOUT = 15
RESOURCE_TIMEOUT = 5
FORBIDDEN_ENV = frozenset({
    "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY", "SSL_CERT_FILE", "SSL_CERT_DIR",
    "SSLKEYLOGFILE", "REQUESTS_CA_BUNDLE", "CURL_CA_BUNDLE", "PYTHONHTTPSVERIFY", "PYTHONHOME",
    "PYTHONPATH", "OPENSSL_CONF", "OPENSSL_MODULES", "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
    "ACTIONS_ID_TOKEN_REQUEST_URL", "GITHUB_TOKEN", "GH_TOKEN",
})
CHILD_ENV = frozenset({"SYSTEMROOT", "WINDIR", "TEMP", "TMP", "LANG", "LC_ALL", "LC_CTYPE"})


def _guard_environment():
    if any(name.upper() in FORBIDDEN_ENV for name in os.environ):
        raise ValueError("acquisition environment refused")


def _tls_context():
    _guard_environment()
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    context.minimum_version = ssl.TLSVersion.TLSv1_2
    context.verify_mode = ssl.CERT_REQUIRED
    context.check_hostname = True
    context.hostname_checks_common_name = False
    context.verify_flags = ssl.VERIFY_X509_STRICT | ssl.VERIFY_X509_TRUSTED_FIRST
    context.keylog_filename = None
    context.load_default_certs(ssl.Purpose.SERVER_AUTH)
    _guard_environment()
    roots = context.get_ca_certs(binary_form=True)
    if not 0 < len(roots) <= 4096 or any(not 0 < len(cert) <= 65536 for cert in roots):
        raise ValueError("platform CA roster refused")
    roster = "".join(sorted(hashlib.sha256(cert).hexdigest() + "\n" for cert in roots)).encode()
    return context, hashlib.sha256(roster).hexdigest()


def _get_public(path, context):
    # Private transport seam for synthetic tests, never an operator URL/CA option.
    if path not in PATHS or not context.check_hostname or context.verify_mode != ssl.CERT_REQUIRED:
        raise ValueError("fixed transport refused")
    _guard_environment()
    deadline = time.monotonic() + RESOURCE_TIMEOUT
    connection = http.client.HTTPSConnection(HOST, 443, timeout=RESOURCE_TIMEOUT, context=context)
    try:
        connection.connect()
        peer = connection.sock.getpeercert(binary_form=True)
        version = connection.sock.version()
        if not peer or len(peer) > 16384 or version not in ("TLSv1.2", "TLSv1.3"):
            raise ValueError("TLS peer observation refused")
        connection.request("GET", path, headers={"Accept": "application/json",
                           "Accept-Encoding": "identity", "Connection": "close"})
        response = connection.getresponse()
        with response:
            if response.status != 200:
                raise ValueError("public response status refused")
            headers = {}
            for name, value in response.getheaders():
                name = name.lower()
                if name in headers:
                    raise ValueError("duplicate response header")
                headers[name] = value
            if ("location" in headers or headers.get("content-encoding", "identity").lower() != "identity"
                    or headers.get("content-type", "").split(";", 1)[0].strip().lower() != "application/json"):
                raise ValueError("public response representation refused")
            transfer = headers.get("transfer-encoding")
            length = headers.get("content-length")
            if transfer is not None and (transfer.lower() != "chunked" or length is not None):
                raise ValueError("ambiguous response framing")
            expected = None
            if length is not None:
                if not length.isascii() or not length.isdecimal() or len(length) > 5:
                    raise ValueError("response length refused")
                expected = int(length)
                if not 0 < expected <= source.MAX_BYTES:
                    raise ValueError("response budget refused")
            body = bytearray()
            while True:
                _guard_environment()
                if time.monotonic() >= deadline:
                    raise ValueError("resource deadline refused")
                chunk = response.read1(min(4096, source.MAX_BYTES + 1 - len(body)))
                if not chunk:
                    break
                body.extend(chunk)
                if len(body) > source.MAX_BYTES:
                    raise ValueError("response budget refused")
            if not body or (expected is not None and expected != len(body)) or time.monotonic() >= deadline:
                raise ValueError("response completion refused")
            return bytes(body), {"url": source.ISSUER + path, "status": 200,
                                 "tls_version": version, "peer_certificate_sha256": hashlib.sha256(peer).hexdigest()}
    finally:
        connection.close()


def _acquire_in_worker():
    context, roots_sha = _tls_context()
    before = time.time_ns()
    discovery, first = _get_public(PATHS[0], context)
    parsed = source.document(discovery)
    if (not isinstance(parsed, dict) or parsed.get("issuer") != source.ISSUER
            or parsed.get("jwks_uri") != source.JWKS_URI
            or parsed.get("id_token_signing_alg_values_supported") != ["RS256"]):
        raise ValueError("fixed discovery refused")
    jwks, second = _get_public(PATHS[1], context)
    inspection = source.inspect_sources(discovery, jwks)
    after = time.time_ns()
    _guard_environment()
    if after < before:
        raise ValueError("acquisition job clock reversed")
    return {"schema_version": SCHEMA, "platform_ca_roster_sha256": roots_sha,
            "received_before_unix_ns": before, "received_after_unix_ns": after,
            "transport": [first, second], "discovery_base64": base64.b64encode(discovery).decode(),
            "jwks_base64": base64.b64encode(jwks).decode(), "inspection": inspection,
            "authenticated_transport_observed": True, "key_selected": False,
            "trust_config_created": False, "protected_job_token_verified": False, "admission_allowed": False}


def _run_worker():
    _guard_environment()
    # Only this exact owned child may be terminated by subprocess.run on timeout.
    result = subprocess.run([sys.executable, "-E", "-s", str(Path(__file__).resolve()), "_worker"],
                            env={k: os.environ[k] for k in os.environ if k.upper() in CHILD_ENV},
                            capture_output=True, timeout=WORKER_TIMEOUT, check=False)
    _guard_environment()
    if result.returncode != 0 or result.stderr or not 0 < len(result.stdout) <= MAX_REPORT_BYTES:
        raise ValueError("owned acquisition child refused")
    report = json.loads(result.stdout, object_pairs_hook=source.unique,
                        parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite report")))
    if (not isinstance(report, dict) or report.get("schema_version") != SCHEMA
            or report.get("authenticated_transport_observed") is not True
            or any(report.get(k) is not False for k in ("key_selected", "trust_config_created",
                                                      "protected_job_token_verified", "admission_allowed"))):
        raise ValueError("owned acquisition report refused")
    expected_fields = {"schema_version", "platform_ca_roster_sha256", "received_before_unix_ns",
                       "received_after_unix_ns", "transport", "discovery_base64", "jwks_base64", "inspection",
                       "authenticated_transport_observed", "key_selected", "trust_config_created",
                       "protected_job_token_verified", "admission_allowed"}
    def digest(value):
        return isinstance(value, str) and len(value) == 64 and all(c in "0123456789abcdef" for c in value)
    if set(report) != expected_fields or not digest(report["platform_ca_roster_sha256"]):
        raise ValueError("report shape refused")
    before, after = report["received_before_unix_ns"], report["received_after_unix_ns"]
    if (type(before) is not int or type(after) is not int or not 0 < before <= after < 1 << 64
            or after - before > WORKER_TIMEOUT * 1_000_000_000):
        raise ValueError("report time refused")
    transport = report["transport"]
    if not isinstance(transport, list) or len(transport) != 2:
        raise ValueError("report transport refused")
    for item, path in zip(transport, PATHS):
        if (not isinstance(item, dict) or set(item) != {"url", "status", "tls_version", "peer_certificate_sha256"}
                or item["url"] != source.ISSUER + path or type(item["status"]) is not int or item["status"] != 200
                or item["tls_version"] not in ("TLSv1.2", "TLSv1.3") or not digest(item["peer_certificate_sha256"])):
            raise ValueError("report transport refused")
    bodies = []
    for field in ("discovery_base64", "jwks_base64"):
        encoded = report[field]
        if not isinstance(encoded, str) or not 0 < len(encoded) <= ((source.MAX_BYTES + 2) // 3) * 4:
            raise ValueError("report body encoding refused")
        raw = base64.b64decode(encoded, validate=True)
        if not 0 < len(raw) <= source.MAX_BYTES or base64.b64encode(raw).decode() != encoded:
            raise ValueError("report body refused")
        bodies.append(raw)
    expected_inspection = source.inspect_sources(*bodies)
    if json.dumps(report["inspection"], sort_keys=True, separators=(",", ":")) != json.dumps(expected_inspection, sort_keys=True, separators=(",", ":")):
        raise ValueError("report inspection drift")
    return report


def acquire_public_sources():
    """No arguments, no automatic enrollment; use only after acquisition approval."""
    return _run_worker()


def main():
    try:
        if sys.argv[1:] == ["_worker"]:
            report = _acquire_in_worker()
        elif sys.argv[1:] == ["acquire-public"]:
            report = acquire_public_sources()
        else:
            raise ValueError("unknown acquisition command")
        raw = json.dumps(report, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        if len(raw) > MAX_REPORT_BYTES:
            raise ValueError("report budget refused")
    except (ValueError, OSError, http.client.HTTPException, subprocess.SubprocessError, RecursionError, TypeError):
        print("Public OIDC acquisition refused; no trust selection or admission created.", file=sys.stderr)
        return 2
    sys.stdout.buffer.write(raw)
    return 0


if __name__ == "__main__":
    sys.exit(main())
