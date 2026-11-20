from contextlib import contextmanager
from hashlib import file_digest
import os
from pathlib import Path
import select
import shutil
import socket
import subprocess
import tempfile
import time
import unittest

from scripts.perf import w11_config_readback_074 as readback
from scripts.perf import w11_product_build_receipt_074 as receipt


@unittest.skipUnless(os.environ.get("W11_READBACK_FIXTURE") is not None, "isolated Linux process fixture")
class NativeReadbackTests(unittest.TestCase):
    def setUp(self):
        self.cwd = Path.cwd().resolve()
        self.fixture = Path(os.environ["W11_READBACK_FIXTURE"]).resolve(strict=True)
        self.assertEqual(self.fixture.parent, self.cwd)
        with self.fixture.open("rb") as stream:
            sha = file_digest(stream, "sha256").hexdigest()
        self.binary = {"sha256": sha, "bytes": self.fixture.stat().st_size}
        self.boot = readback.BOOT_ID.read_text().strip()

    @contextmanager
    def original(self, selector=None, *, readback_enabled=True, entries=None, copy=False):
        temporary = tempfile.TemporaryDirectory(prefix="rd-", dir=self.cwd)
        directory = Path(temporary.name).resolve()
        self.assertEqual(directory.parent, self.cwd)
        with temporary, readback.private_channel(directory) as channel:
            image = self.fixture
            if copy:
                image = directory / "copied-elf"
                shutil.copy2(self.fixture, image)
            environment = {"PATH": "/usr/bin:/bin", "LANG": "C"}
            if readback_enabled:
                environment["HYDRACACHE_W11_CONFIG_READBACK"] = "1"
            if selector is not None:
                environment["HYDRACACHE_CLIENT_BATCH_PROFILE"] = selector
            if entries is None:
                entries = 128 if selector is None else 256
            deadline = time.monotonic() + 60
            with subprocess.Popen(
                [str(image), str(entries)], cwd=directory, env=environment,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            ) as child:
                try:
                    events = select.poll()
                    events.register(child.stdout, select.POLLIN)
                    self.assertTrue(events.poll(5000), "fixture readiness timed out")
                    ready = child.stdout.readline(256)
                    yield directory, channel, child, ready, deadline
                finally:
                    if child.poll() is None:
                        try:
                            child.communicate(b"x", timeout=5)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.communicate(timeout=5)
                            raise

    def watch(self, child):
        return readback.observe_child(child, self.fixture, self.binary, self.boot)

    def test_default_and_opt_in_are_bound_to_the_original_daemon(self):
        rows = {}
        for selector in (None, "w11-256"):
            with self.original(selector) as (_, channel, child, ready, end):
                with self.watch(child) as observe:
                    value = readback.read_configuration(channel, observe, selector, ready, end)
                    self.assertEqual(value.configuration, receipt.config(selector))
                    self.assertEqual(value.generation, observe())
                    self.assertEqual(value.generation["pid"], child.pid)
                    self.assertEqual(value.generation["elf_sha256"], self.binary["sha256"])
                    if selector:
                        self.assertEqual(value.sha256, receipt.PROFILE_SHA256)
                    rows[selector or "default"] = {
                        "configuration": value.configuration, "generation": value.generation,
                        "config_sha256": value.sha256, "ready_hex": ready.hex(),
                    }
                    target = self.cwd / f"{selector or 'default'}.config.json"
                    with target.open("xb") as stream:
                        stream.write(value.raw)
            self.assertEqual(child.returncode, 0)
        self.assertNotEqual(rows["default"]["generation"], rows["w11-256"]["generation"])
        with (self.cwd / "config-observations.json").open("xb") as out:
            out.write(receipt.canonical(rows) + b"\n")

    def test_normal_readiness_has_no_readback_when_option_is_absent(self):
        with self.original(readback_enabled=False) as (_, channel, child, ready, _):
            self.assertEqual(ready, readback.READY)
            with self.watch(child) as observe:
                self.assertEqual(observe()["pid"], child.pid)
                events = select.poll()
                events.register(channel, select.POLLIN)
                self.assertFalse(events.poll(0))

    def test_identical_payload_from_other_process_is_refused(self):
        with self.original(readback_enabled=False) as (directory, channel, child, ready, end):
            with self.watch(child) as observe, socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as sender:
                sender.sendto(receipt.canonical(receipt.config(None)), str(directory / readback.SOCKET))
                with self.assertRaisesRegex(ValueError, "sender differs"):
                    readback.read_configuration(channel, observe, None, ready, end)

    def test_same_bytes_from_different_installation_are_refused(self):
        with self.original(copy=True) as (_, _, child, _, _):
            with self.assertRaisesRegex(ValueError, "different executable"):
                with self.watch(child):
                    self.fail("a different installed inode was observed")

    def test_selector_alone_cannot_establish_effective_limits(self):
        with self.original("w11-256", entries=128) as (_, channel, child, ready, end):
            with self.watch(child) as observe:
                with self.assertRaisesRegex(ValueError, "differs from approved limits"):
                    readback.read_configuration(channel, observe, "w11-256", ready, end)

    def test_dead_process_cannot_provide_a_generation(self):
        with self.original() as (_, _, child, _, _):
            with self.watch(child) as observe:
                child.communicate(b"x", timeout=5)
                self.assertEqual(child.returncode, 0)
                with self.assertRaisesRegex(ValueError, "original daemon exited"):
                    observe()
