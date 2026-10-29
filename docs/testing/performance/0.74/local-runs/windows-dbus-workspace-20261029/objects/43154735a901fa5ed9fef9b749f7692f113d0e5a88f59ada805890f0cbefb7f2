"""Local modeled acquisition and real synthetic loopback TLS, never GitHub or production trust."""
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
import io
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
import sys
import threading
import time
from unittest import TestCase, main
from unittest.mock import patch
from scripts.perf import diagnostic_oidc_acquisition_074 as acquisition
from scripts.perf.test_diagnostic_oidc_source_074 import fixtures, raw


class Response:
    def __init__(self, body=b"{}", headers=None, status=200, error=None):
        self.body = io.BytesIO(body)
        self.status = status
        self.headers = headers if headers is not None else [("Content-Type", "application/json"), ("Content-Length", str(len(body)))]
        self.closed = False
        self.error = error

    def getheaders(self):
        return self.headers

    def read1(self, size):
        if self.error:
            raise self.error
        return self.body.read(size)

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.closed = True


class Connection:
    def __init__(self, response, error=None):
        self.response, self.error, self.closed, self.requests = response, error, False, []
        self.sock = self

    def connect(self):
        if self.error:
            raise self.error

    def getpeercert(self, binary_form):
        assert binary_form
        return b"synthetic-modeled-certificate-not-a-real-peer"

    def version(self):
        return "TLSv1.3"

    def request(self, *args, **kwargs):
        self.requests.append((args, kwargs))

    def getresponse(self):
        return self.response

    def close(self):
        self.closed = True


def modeled_report():
    discovery, jwks = fixtures()
    def receive(path, _context):
        body = raw(discovery if path == acquisition.PATHS[0] else jwks)
        return body, {"url": acquisition.source.ISSUER + path, "status": 200,
                      "tls_version": "TLSv1.3", "peer_certificate_sha256": "a" * 64}
    with patch.object(acquisition, "_get_public", side_effect=receive):
        return acquisition._acquire_in_worker()


@contextmanager
def tls_fixture():
    # The committed key is explicitly public synthetic fixture material.
    directory = Path(__file__).parent / "fixtures"
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(directory / "public-synthetic-oidc-tls-server.pem", directory / "public-synthetic-oidc-tls-key.pem")
    listener = socket.socket()
    listener.bind(("127.0.0.1", 0))
    listener.listen(1)
    listener.settimeout(5)
    port = listener.getsockname()[1]
    observations, errors = [], []
    def server():
        try:
            with listener:
                client, _ = listener.accept()
                with client:
                    client.settimeout(5)
                    with context.wrap_socket(client, server_side=True) as secured:
                        request = bytearray()
                        while b"\r\n\r\n" not in request:
                            data = secured.recv(1024)
                            if not data or len(request) + len(data) > 4096:
                                raise AssertionError("fixture request incomplete or over budget")
                            request.extend(data)
                        observations.append(bytes(request))
                        secured.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}')
        except (ssl.SSLError, ConnectionResetError) as error:
            # Windows may reset instead of delivering the peer's TLS alert.
            errors.append(error if observations else "synthetic handshake refused")
        except Exception as error:
            errors.append(error)
    thread = threading.Thread(target=server, daemon=True)
    thread.start()
    original_connect = socket.create_connection
    def loopback(address, timeout=None, source_address=None, **kwargs):
        assert address == (acquisition.HOST, 443)
        return original_connect(("127.0.0.1", port), timeout, source_address, **kwargs)
    try:
        with patch.object(socket, "create_connection", side_effect=loopback):
            yield observations, errors
    finally:
        thread.join(6)
        listener.close()
        assert not thread.is_alive(), "owned loopback helper did not exit"
        assert all(isinstance(error, str) for error in errors), errors


class AcquisitionTests(TestCase):
    def setUp(self):
        self.environment = patch.dict(os.environ, {}, clear=True)
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def test_platform_context_keeps_chain_hostname_san_and_no_keylog(self):
        context, digest = acquisition._tls_context()
        self.assertEqual(context.verify_mode, ssl.CERT_REQUIRED)
        self.assertTrue(context.check_hostname)
        self.assertFalse(context.hostname_checks_common_name)
        self.assertGreaterEqual(context.minimum_version, ssl.TLSVersion.TLSv1_2)
        self.assertTrue(context.verify_flags & ssl.VERIFY_X509_STRICT)
        self.assertFalse(context.verify_flags & ssl.VERIFY_X509_PARTIAL_CHAIN)
        self.assertIsNone(context.keylog_filename)
        self.assertEqual(len(digest), 64)

    def test_environment_refuses_before_launch_or_ca_loading_without_echo(self):
        for name in acquisition.FORBIDDEN_ENV:
            for spelling in [name, name.lower()]:
                with patch.dict(os.environ, {spelling: "synthetic-do-not-echo"}, clear=True), patch.object(subprocess, "run") as launch:
                    with self.assertRaises(ValueError):
                        acquisition.acquire_public_sources()
                    launch.assert_not_called()
                    with self.assertRaises(ValueError):
                        acquisition._tls_context()
        with patch.object(sys, "argv", ["tool", "acquire-public"]), patch.dict(os.environ, {"GH_TOKEN": "synthetic-do-not-echo"}), patch.object(sys, "stderr", io.StringIO()) as errors:
            self.assertEqual(acquisition.main(), 2)
            self.assertNotIn("synthetic-do-not-echo", errors.getvalue())

    def test_fixed_http_request_accepts_bounded_identity_and_chunked_sources(self):
        context, _ = acquisition._tls_context()
        for headers in [[("Content-Type", "application/json; charset=utf-8")],
                        [("Content-Type", "application/json"), ("Transfer-Encoding", "chunked")],
                        [("Content-Type", "application/json"), ("Content-Length", "2")]]:
            response = Response(headers=headers)
            connection = Connection(response)
            with patch.object(acquisition.http.client, "HTTPSConnection", return_value=connection) as create:
                body, meta = acquisition._get_public(acquisition.PATHS[0], context)
            create.assert_called_once_with(acquisition.HOST, 443, timeout=5, context=context)
            self.assertEqual(body, b"{}")
            self.assertTrue(connection.closed and response.closed)
            self.assertEqual(connection.requests[0][0], ("GET", acquisition.PATHS[0]))
            self.assertNotIn("Authorization", connection.requests[0][1]["headers"])
            self.assertEqual(meta["url"], acquisition.source.ISSUER + acquisition.PATHS[0])

    def test_http_status_framing_compression_budget_and_partial_body_refuse(self):
        context, _ = acquisition._tls_context()
        good = [("Content-Type", "application/json")]
        # Keep mutation cases explicit rather than accepting alternate representations.
        responses = [Response(status=status) for status in [301, 302, 307, 401, 429, 500]]
        for headers in [good + [("Location", "https://foreign.invalid")], good + [("Content-Encoding", "gzip")],
                        good + [("Content-Type", "application/json")], [("Content-Type", "text/html")],
                        good + [("Content-Length", "2"), ("content-length", "2")],
                        good + [("Transfer-Encoding", "chunked"), ("Content-Length", "2")],
                        good + [("Transfer-Encoding", "gzip")]]:
            responses.append(Response(headers=headers))
        responses.extend(Response(headers=good + [("Content-Length", v)]) for v in ["0", "-1", "65537", "1e2", "１２", "100000", "3"])
        responses.extend([Response(body=b""), Response(body=b"x" * 65537, headers=good),
                          Response(error=TimeoutError("synthetic-network-error"))])
        for response in responses:
            connection = Connection(response)
            with patch.object(acquisition.http.client, "HTTPSConnection", return_value=connection):
                with self.assertRaises((ValueError, OSError)):
                    acquisition._get_public(acquisition.PATHS[0], context)
            self.assertTrue(connection.closed)
        with patch.object(acquisition.http.client, "HTTPSConnection") as create:
            with self.assertRaises(ValueError):
                acquisition._get_public("/foreign", context)
            create.assert_not_called()

    def test_tls_and_resource_deadline_failures_close_exact_connection(self):
        context, _ = acquisition._tls_context()
        for error in [ssl.SSLCertVerificationError("synthetic failure"), TimeoutError("synthetic timeout")]:
            connection = Connection(Response(), error)
            with patch.object(acquisition.http.client, "HTTPSConnection", return_value=connection):
                with self.assertRaises(OSError):
                    acquisition._get_public(acquisition.PATHS[0], context)
            self.assertTrue(connection.closed)
        connection = Connection(Response())
        with patch.object(acquisition.http.client, "HTTPSConnection", return_value=connection), patch.object(time, "monotonic", side_effect=[0, 5]):
            with self.assertRaises(ValueError):
                acquisition._get_public(acquisition.PATHS[0], context)
        self.assertTrue(connection.closed)
        for version, peer in [("TLSv1.1", b"model"), ("TLSv1.3", b""), ("TLSv1.3", b"x" * 16385)]:
            connection = Connection(Response())
            with patch.object(acquisition.http.client, "HTTPSConnection", return_value=connection), patch.object(connection, "version", return_value=version), patch.object(connection, "getpeercert", return_value=peer):
                with self.assertRaises(ValueError):
                    acquisition._get_public(acquisition.PATHS[0], context)
            self.assertTrue(connection.closed)

    def test_worker_malformed_discovery_stops_before_jwks_and_clock_reversal_refuses(self):
        with patch.object(acquisition, "_get_public", return_value=(b"{}", {})) as receive:
            with self.assertRaises(ValueError):
                acquisition._acquire_in_worker()
            self.assertEqual(receive.call_count, 1)
        with patch.object(time, "time_ns", side_effect=[100, 99]):
            with self.assertRaises(ValueError):
                modeled_report()

    def test_parent_exact_isolated_child_and_modeled_report_keep_all_admission_closed(self):
        report = modeled_report()
        result = subprocess.CompletedProcess([], 0, raw(report), b"")
        with patch.object(subprocess, "run", return_value=result) as launch, patch.dict(os.environ, {"UNRELATED_SECRET_NOT_REAL": "not-propagated", "LANG": "C"}):
            actual = acquisition.acquire_public_sources()
        self.assertEqual(actual, report)
        args, kwargs = launch.call_args
        self.assertEqual(args[0], [sys.executable, "-E", "-s", str(Path(acquisition.__file__).resolve()), "_worker"])
        self.assertEqual(kwargs["timeout"], 15)
        self.assertEqual(kwargs["env"], {"LANG": "C"})
        self.assertFalse(actual["key_selected"] or actual["admission_allowed"])
        self.assertTrue(actual["inspection"]["offline_parser_only"])

    def test_parent_child_timeout_failure_partial_and_inconsistent_report_refuse(self):
        report = modeled_report()
        cases = [(1, raw(report), b""), (0, raw(report), b"warning"), (0, b"", b""),
                 (0, b"x" * (acquisition.MAX_REPORT_BYTES + 1), b""), (0, b"{", b"")]
        for field, value in [("schema_version", "future"), ("admission_allowed", True), ("key_selected", None),
                             ("received_before_unix_ns", True), ("received_after_unix_ns", 1),
                             ("discovery_base64", "bad!"), ("inspection", {}), ("transport", []), ("extra", True)]:
            changed = {**report, field: value}
            cases.append((0, raw(changed), b""))
        for code, stdout, stderr in cases:
            with patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], code, stdout, stderr)):
                with self.assertRaises(ValueError):
                    acquisition.acquire_public_sources()
        changed = json.loads(raw(report))
        changed["inspection"]["admission_allowed"] = 0
        with patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], 0, raw(changed), b"")):
            with self.assertRaises(ValueError):
                acquisition.acquire_public_sources()
        with patch.object(subprocess, "run", side_effect=subprocess.TimeoutExpired("synthetic-child", 15)):
            with self.assertRaises(subprocess.TimeoutExpired):
                acquisition.acquire_public_sources()

    def test_actual_owned_child_timeout_reaps_only_fixture_and_never_accepts_output(self):
        real_run, real_popen = subprocess.run, subprocess.Popen
        children = []
        def launch(*args, **kwargs):
            child = real_popen(*args, **kwargs)
            children.append(child)
            return child
        def fixture_run(_command, **kwargs):
            return real_run([sys.executable, "-c", "import time; print('partial-model-output', flush=True); time.sleep(60)"], **kwargs)
        with patch.object(subprocess, "Popen", side_effect=launch), patch.object(subprocess, "run", side_effect=fixture_run), patch.object(acquisition, "WORKER_TIMEOUT", 0.5):
            with self.assertRaises(subprocess.TimeoutExpired):
                acquisition.acquire_public_sources()
        self.assertEqual(len(children), 1)
        self.assertIsNotNone(children[0].poll())
        print("owned_fixture_timeout=reaped partial_output=refused actual_github_fetch=false")

    def test_cli_modeled_success_and_budget_never_write_configuration(self):
        report = modeled_report()
        class Output:
            def __init__(self):
                self.buffer = io.BytesIO()
        for command, function in [("acquire-public", "acquire_public_sources"), ("_worker", "_acquire_in_worker")]:
            output = Output()
            with patch.object(sys, "argv", ["tool", command]), patch.object(acquisition, function, return_value=report), patch.object(sys, "stdout", output):
                self.assertEqual(acquisition.main(), 0)
            self.assertEqual(json.loads(output.buffer.getvalue()), report)
            self.assertTrue(output.buffer.getvalue().endswith(b"\n"))
        output = Output()
        with patch.object(sys, "argv", ["tool", "acquire-public"]), patch.object(acquisition, "acquire_public_sources", return_value={"padding": "x" * acquisition.MAX_REPORT_BYTES}), patch.object(sys, "stdout", output), patch.object(sys, "stderr", io.StringIO()):
            self.assertEqual(acquisition.main(), 2)
        self.assertEqual(output.buffer.getvalue(), b"")

    def test_actual_loopback_tls_accept_and_hostname_or_unknown_ca_refuse(self):
        context, _ = acquisition._tls_context()
        context.load_verify_locations(cafile=str(Path(__file__).parent / "fixtures/public-synthetic-oidc-tls-ca.pem"))
        with tls_fixture() as (requests, errors):
            body, meta = acquisition._get_public(acquisition.PATHS[0], context)
        self.assertEqual(body, b"{}")
        self.assertFalse(errors)
        self.assertIn(b"Host: token.actions.githubusercontent.com\r\n", requests[0])
        self.assertIn(meta["tls_version"], ("TLSv1.2", "TLSv1.3"))
        for unknown_ca in [False, True]:
            client, _ = acquisition._tls_context()
            if not unknown_ca:
                client.load_verify_locations(cafile=str(Path(__file__).parent / "fixtures/public-synthetic-oidc-tls-ca.pem"))
            # The negative hostname changes only the private local test globals.
            with patch.object(acquisition, "HOST", acquisition.HOST if unknown_ca else "wrong.synthetic.invalid"):
                with tls_fixture() as (requests, errors):
                    with self.assertRaises(ssl.SSLCertVerificationError):
                        acquisition._get_public(acquisition.PATHS[0], client)
                self.assertFalse(requests)
                self.assertEqual(errors, ["synthetic handshake refused"])
        print("actual_loopback_tls=accept,wrong_hostname,unknown_ca actual_github_fetch=false")

    def test_seeded_reports_independent_acquisitions_and_cli_unknown_refuse(self):
        seed = 0x07572074
        report = modeled_report()
        for _ in range(32):
            seed ^= (seed << 13) & ((1 << 64) - 1)
            seed ^= seed >> 7
            seed ^= (seed << 17) & ((1 << 64) - 1)
            changed = json.loads(raw(report))
            changed["transport"][seed % 2]["peer_certificate_sha256"] = "!" + "a" * 63
            with patch.object(subprocess, "run", return_value=subprocess.CompletedProcess([], 0, raw(changed), b"")):
                with self.assertRaises(ValueError):
                    acquisition.acquire_public_sources()
        with ThreadPoolExecutor(max_workers=4) as executor:
            with patch.object(acquisition, "_get_public", side_effect=lambda path, _: (raw(fixtures()[0 if path == acquisition.PATHS[0] else 1]),
                {"url": acquisition.source.ISSUER + path, "status": 200, "tls_version": "TLSv1.3", "peer_certificate_sha256": "a" * 64})):
                results = list(executor.map(lambda _: acquisition._acquire_in_worker(), range(4)))
        self.assertEqual(len(results), 4)
        self.assertTrue(all(not r["key_selected"] and not r["admission_allowed"] for r in results))
        result = subprocess.run([sys.executable, str(Path(acquisition.__file__).resolve()), "--url=not-real-secret"], capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 2)
        self.assertFalse(result.stdout)
        self.assertNotIn(b"not-real-secret", result.stderr)
        print("seed=0x07572074 report_mutations=32 independent_models=4 production_authority=false")


if __name__ == "__main__":
    main()
