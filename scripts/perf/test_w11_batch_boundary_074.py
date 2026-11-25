from contextlib import contextmanager
from hashlib import sha256
import io
import os
from pathlib import Path
import shutil
import socket
import ssl
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from scripts.perf import w11_batch_boundary_074 as collect
from scripts.perf import w11_product_build_receipt_074 as receipt


BOOT = "23d11511-fdd4-451c-8200-faec1091a2ee"
VERSION = b"0.74.0-rc.1"
TOKEN = b"5e" * 24
SOURCE = ("1" * 40, "2" * 40)
LOCK = "0" * 64
WORKFLOW = receipt.build.approved_workflow_run("a" * 40, "123", "1")
IMAGE = b"\x7fELF\x02\x01\x01" + bytes(9) + b"\x03\x00\x3e\x00" + bytes(53)
HELLO = (b"%7\r\n+id\r\n:0\r\n+version\r\n+" + VERSION
         + b"\r\n+server\r\n+hydracache\r\n+proto\r\n:3\r\n"
           b"+role\r\n+master\r\n+mode\r\n+standalone\r\n+modules\r\n*0\r\n")
OK = b"+OK\r\n"
REFUSED = b"-ERR HydraCache client error: request too large\r\n"
NO_AUTH = b"-NOAUTH Authentication required.\r\n"
BAD_AUTH = b"-WRONGPASS invalid username-password pair or user is disabled.\r\n"


def read_command(stream):
    line = stream.readline(32)
    if not line:
        return None
    assert line[:1] == b"*" and line[-2:] == b"\r\n"
    values = []
    for _ in range(int(line[1:-2])):
        header = stream.readline(32)
        assert header[:1] == b"$" and header[-2:] == b"\r\n"
        size = int(header[1:-2])
        body = stream.read(size)
        assert len(body) == size and stream.read(2) == b"\r\n"
        values.append(body)
    return values


def bulk(value):
    return b"_\r\n" if value is None else b"$" + str(len(value)).encode() + b"\r\n" + value + b"\r\n"


class DaemonModel:
    def __init__(self, limit, token=TOKEN, *, fault=None):
        self.limit = limit
        self.token = token
        self.fault = fault
        self.data = {}
        self.operations = []
        self.missing_cert = 0

    def responses(self, stream):
        authenticated = False
        while True:
            parts = read_command(stream)
            assert parts is not None
            name, *args = parts
            if name == b"QUIT":
                yield OK
                return
            if name in (b"HELLO", b"AUTH"):
                required = [b"default", self.token]
                if name == b"HELLO":
                    required = [b"3", b"AUTH", *required]
                authenticated = args == required
                yield (HELLO if name == b"HELLO" else OK) if authenticated else BAD_AUTH
                continue
            if not authenticated:
                yield NO_AUTH
                continue
            self.operations.append((name, len(args)))
            if name == b"SET":
                key, value = args
                self.data[key] = value
                yield OK
            elif name == b"GET":
                yield bulk(self.data.get(args[0]))
            elif name == b"MSET":
                entries = tuple(zip(args[::2], args[1::2]))
                if len(entries) > self.limit:
                    if self.fault == "partial":
                        self.data.update(entries[:1])
                    yield REFUSED
                else:
                    self.data.update(entries)
                    yield OK
            elif name == b"MGET":
                if len(args) > self.limit:
                    yield REFUSED
                else:
                    if self.fault == "order" and len(args) == 256:
                        args[0], args[1] = args[1], args[0]
                    yield b"*" + str(len(args)).encode() + b"\r\n" + b"".join(
                        bulk(self.data.get(key)) for key in args)
            else:
                raise AssertionError(name)

    def exchange(self, material, address, request, deadline, certificate=True):
        if not certificate:
            self.missing_cert += 1
            if self.fault != "no-mtls":
                return b""
        reply = b"".join(self.responses(io.BytesIO(request)))
        assert len(reply) <= 4096
        return reply


@contextmanager
def tls_daemon(material, model):
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.verify_mode = ssl.CERT_REQUIRED
    context.load_verify_locations(material / "ca.crt")
    context.load_cert_chain(material / "server.crt", material / "server.key")
    stop = threading.Event()
    failures = []
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        listener.settimeout(0.2)

        def serve():
            try:
                while not stop.is_set():
                    try:
                        conn, _ = listener.accept()
                    except TimeoutError:
                        continue
                    with conn:
                        conn.settimeout(5)
                        with context.wrap_socket(
                            conn, server_side=True, do_handshake_on_connect=False
                        ) as tls:
                            try:
                                tls.do_handshake()
                            except ssl.SSLError as error:
                                if error.reason != "PEER_DID_NOT_RETURN_A_CERTIFICATE":
                                    raise
                                model.missing_cert += 1
                                # Let the client's read receive the pending TLS alert
                                # before closing the transport with unread ciphertext.
                                with socket.socket(fileno=tls.detach()) as rejected:
                                    rejected.settimeout(5)
                                    rejected.shutdown(socket.SHUT_WR)
                                    total = 0
                                    while chunk := rejected.recv(1024):
                                        total += len(chunk)
                                        assert total <= 4096
                                continue
                            with tls.makefile("rb") as stream:
                                for reply in model.responses(stream):
                                    tls.sendall(reply)
            except Exception as error:
                failures.append(error)

        worker = threading.Thread(target=serve, daemon=True)
        worker.start()
        try:
            yield listener.getsockname()
        finally:
            stop.set()
            worker.join(timeout=6)
            assert not worker.is_alive()
            if failures:
                raise failures[0]


class BoundaryTests(unittest.TestCase):
    def setUp(self):
        base = Path.cwd().resolve()
        temporary = tempfile.TemporaryDirectory(prefix="batch-cases-", dir=base)
        self.root = Path(temporary.name).resolve()
        self.assertEqual(self.root.parent, base)
        self.addCleanup(temporary.cleanup)
        self.archives = self.root / "archives"
        self.archives.mkdir()
        self.artifact = self.root / "artifact"
        self.artifact.mkdir()
        (self.artifact / "I74-server").write_bytes(IMAGE)
        self.subject = receipt.build.receipt_for_source("I74", self.artifact, SOURCE, LOCK, WORKFLOW)
        self.subject["schema"] = receipt.RECEIPT_SCHEMA

    def observe(self, selector):
        pid = 817 if selector is None else 823
        return {"boot_id": BOOT, "pid": pid, "start_ticks": pid * 3,
                "elf_sha256": self.subject["binary"]["sha256"]}

    def run_profile(self, selector, model, observer=None):
        session = collect.RespSession(self.root, ("127.0.0.1", 1), TOKEN, time.monotonic() + 60, VERSION)
        with patch.object(collect.probe, "exchange", model.exchange), patch.object(
                collect.probe, "disk_guard", return_value=collect.probe.DISK_FLOOR + (1 << 26)):
            return collect.collect_profile(self.subject, selector, receipt.config(selector),
                                           observer or (lambda: self.observe(selector)),
                                           session, self.archives / (selector or collect.DEFAULT))

    def publish(self, default, opt_in):
        with patch.object(collect.probe, "disk_guard", return_value=collect.probe.DISK_FLOOR + (1 << 26)):
            return collect.publish_evidence(self.subject, default, opt_in, self.archives, self.artifact)

    def accept_for_signature_review(self, proof):
        role = receipt.ApprovedRole("I74", *SOURCE, LOCK, proof)
        actual = receipt.expected_receipt(role, self.artifact, WORKFLOW)
        self.assertEqual(actual["w11_profile"]["boundary_evidence_sha256"], proof)
        self.assertEqual(actual["w11_profile"]["accepted"], [128, 129, 256])
        self.assertEqual(actual["w11_profile"]["refused"], [257])

    def test_default_and_opt_in_publish_twelve_exact_case_archives(self):
        baseline, candidate = DaemonModel(128), DaemonModel(256)
        default = self.run_profile(None, baseline)
        opt_in = self.run_profile("w11-256", candidate)
        self.assertEqual((len(default["cases"]), len(opt_in["cases"])), (4, 8))
        self.assertEqual((baseline.missing_cert, candidate.missing_cert), (1, 1))
        self.accept_for_signature_review(self.publish(default, opt_in))
        for directory in self.archives.glob("*/*"):
            if directory.name == "auth":
                self.assertEqual(len(list(directory.iterdir())), 4)
                continue
            raw = (directory / "boundary.request.resp").read_bytes()
            parts = read_command(io.BytesIO(raw))
            self.assertNotIn(TOKEN, raw)
            keys = parts[1::2] if parts[0] == b"MSET" else parts[1:]
            self.assertEqual(len(set(keys)), int(directory.name.split("-")[1]))
            self.assertTrue(all(len(key) == 16 for key in keys))
            if parts[0] == b"MSET":
                self.assertTrue(all(len(value) == 16 and value.startswith(b"\0\xff\r\n")
                                    for value in parts[2::2]))
            reply = (directory / "boundary.response.resp").read_bytes()
            self.assertLessEqual(len(HELLO) + len(reply) + len(OK), 4096)

    def test_refused_mset_cannot_hide_partial_writes(self):
        with self.assertRaisesRegex(ValueError, "after-0 differs"):
            self.run_profile(None, DaemonModel(128, fault="partial"))
        case = self.archives / "default" / "MSET-129"
        self.assertEqual((case / "boundary.response.resp").read_bytes(), REFUSED)
        self.assertTrue((case / "after-0.response.resp").is_file())
        self.assertFalse((case / "oracle.json").exists())

    def test_reordered_mget_and_legacy_opt_in_limit_fail(self):
        with self.assertRaisesRegex(ValueError, "boundary differs"):
            self.run_profile("w11-256", DaemonModel(256, fault="order"))
        self.assertTrue((self.archives / "w11-256" / "MGET-256" / "boundary.response.resp").is_file())
        other = self.root / "other-archives"
        other.mkdir()
        self.archives = other
        with self.assertRaisesRegex(ValueError, "boundary differs"):
            self.run_profile("w11-256", DaemonModel(128))
        self.assertEqual(
            (other / "w11-256" / "MGET-129" / "boundary.response.resp").read_bytes(), REFUSED)

    def test_authentication_drift_stops_before_the_first_case(self):
        with self.assertRaisesRegex(ValueError, "missing client certificate"):
            self.run_profile(None, DaemonModel(128, fault="no-mtls"))
        self.assertEqual({p.name for p in (self.archives / "default").iterdir()}, {"auth"})

    def test_new_generation_stops_before_batch_commands(self):
        before = self.observe(None)
        snapshots = iter((before, {**before, "start_ticks": before["start_ticks"] + 1}))
        with self.assertRaisesRegex(ValueError, "generation changed"):
            self.run_profile(None, DaemonModel(128), observer=lambda: next(snapshots))
        self.assertEqual({p.name for p in (self.archives / "default").iterdir()}, {"auth"})

    def test_changed_transcript_or_authentication_receipt_prevents_publication(self):
        default = self.run_profile(None, DaemonModel(128))
        opt_in = self.run_profile("w11-256", DaemonModel(256))
        wire = self.archives / "w11-256" / "MGET-256" / "boundary.response.resp"
        original = wire.read_bytes()
        wire.write_bytes(original + b"x")
        with self.assertRaisesRegex(ValueError, "length"):
            self.publish(default, opt_in)
        wire.write_bytes(original)
        auth = self.archives / "default" / "auth" / "denied.resp"
        auth.write_bytes(auth.read_bytes() + OK)
        with self.assertRaisesRegex(ValueError, "oracle changed"):
            self.publish(default, opt_in)
        self.assertFalse((self.artifact / receipt.EVIDENCE).exists())

    @unittest.skipUnless(os.environ.get("W11_BATCH_TLS_FIXTURE") == "1", "isolated Linux TLS fixture")
    def test_native_mtls_and_auth_carry_all_boundary_cases(self):
        material = self.root / "material"
        token = collect.probe.fixture(material)
        results = []
        for selector, limit in [(None, 128), ("w11-256", 256)]:
            model = DaemonModel(limit, token)
            with tls_daemon(material, model) as address:
                session = collect.RespSession(material, address, token, time.monotonic() + 60, VERSION)
                results.append(collect.collect_profile(
                    self.subject, selector, receipt.config(selector), lambda: self.observe(selector),
                    session, self.archives / (selector or collect.DEFAULT)))
            self.assertEqual(model.missing_cert, 1)
        proof = collect.publish_evidence(self.subject, *results, self.archives, self.artifact)
        self.accept_for_signature_review(proof)
        destination = os.environ.get("W11_BATCH_TLS_OUTPUT")
        if destination:
            target = Path(destination).resolve()
            self.assertEqual(target.parent, self.root.parent)
            target.mkdir(mode=0o700)
            shutil.copytree(self.archives, target / "archives")
            with (target / receipt.EVIDENCE).open("xb") as out:
                out.write((self.artifact / receipt.EVIDENCE).read_bytes())
