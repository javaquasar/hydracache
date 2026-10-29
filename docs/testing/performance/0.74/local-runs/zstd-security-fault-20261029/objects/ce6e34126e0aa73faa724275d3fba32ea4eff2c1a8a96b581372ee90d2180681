import ctypes
import os
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import Mock, patch

from attempt_output_binding import AttemptOutputs, WindowsFiles
from owned_process_observer import Refusal
from target_result_join import ExpectedSource


class MemoryFiles:
    def __init__(self):
        self.handles, self.nodes, self.closed = {}, {}, []
        self.next_id = 0
        self.read_calls = 0
        self.writer = self.bad_close = None

    def open(self, path, directory=False, sealed=False):
        name = str(path)
        if sealed and Path(name).name == self.writer:
            raise Refusal("existing writer sharing violation")
        if name not in self.nodes:
            self.nodes[name] = dict(volume=1, index=len(self.nodes) + 1, attributes=16 if directory else 0,
                                    links=1, size=0, written=1, data=b"{}")
        self.next_id += 1
        self.handles[self.next_id] = self.nodes[name].copy()
        return self.next_id

    def info(self, handle):
        return {key: value for key, value in self.handles[handle].items() if key != "data"}

    def read(self, handle, limit):
        self.read_calls += 1
        return self.handles[handle]["data"]

    def close(self, handle):
        self.closed.append(handle)
        if handle == self.bad_close:
            raise Refusal("injected output close failure")


def original_custody():
    owner = Mock(pid=17)
    owner.poll.return_value = None
    return SimpleNamespace(owner=owner, root=SimpleNamespace(pid=17, handle=9, samples=[dict(creation_100ns=100)]),
                           exit_proof=None, custody_refusal=None)


class OutputModelTests(unittest.TestCase):
    def fixture(self, temp):
        root = Path(temp)
        api = MemoryFiles()
        outputs = AttemptOutputs("fixed-attempt", root, root / "parent", ExpectedSource("a" * 40, "b" * 64), api)
        c = original_custody()
        outputs.attach(c)
        c.owner.poll.return_value = 0
        c.exit_proof = dict(owner_returncode=0, witnesses=[dict(pid=17, creation_100ns=100, exit_100ns=200)])
        return outputs, c, api

    def test_bound_identity_and_all_files_read_once(self):
        with tempfile.TemporaryDirectory() as temp:
            outputs, c, api = self.fixture(temp)
            raw, binding = outputs.seal(c)
            self.assertEqual(set(raw), set(outputs.LIMITS))
            self.assertEqual(binding["owner"], dict(pid=17, creation_100ns=100))
            self.assertEqual(api.read_calls, 4)
            self.assertFalse(binding["all_writers_proven_stopped"])
            with self.assertRaises(Refusal):
                outputs.seal(c)
            outputs.close()

    def test_stale_slot_and_invalid_label_refuse(self):
        with tempfile.TemporaryDirectory() as temp:
            (Path(temp) / "fixed-attempt").mkdir()
            for label in ("fixed-attempt", "../escape"):
                with self.assertRaises(Refusal):
                    AttemptOutputs(label, temp, temp, ExpectedSource("a" * 40, "b" * 64), MemoryFiles())

    def test_attachment_requires_same_live_owner_and_birth(self):
        for fault in ("exited", "pid", "birth", "handle"):
            with tempfile.TemporaryDirectory() as temp:
                outputs = AttemptOutputs("fixed-attempt", temp, temp, None, MemoryFiles())
                c = original_custody()
                if fault == "exited":
                    c.owner.poll.return_value = 0
                elif fault == "pid":
                    c.owner.pid = 18
                elif fault == "birth":
                    c.root.samples[0]["creation_100ns"] = 0
                else:
                    c.root.handle = None
                with self.assertRaises(Refusal):
                    outputs.attach(c)
                c.owner.poll.return_value = 0
                outputs.close()

    def test_live_owner_prevents_close_or_seal(self):
        with tempfile.TemporaryDirectory() as temp:
            outputs, c, _ = self.fixture(temp)
            c.owner.poll.return_value = None
            c.exit_proof = None
            with self.assertRaisesRegex(Refusal, "live original owner"):
                outputs.close()
            with self.assertRaisesRegex(Refusal, "terminal custody"):
                outputs.seal(c)
            c.owner.poll.return_value = 0
            outputs.close()

    def test_foreign_custody_or_witness_cannot_select_outputs(self):
        for fault in ("foreign", "pid", "birth", "status", "query"):
            with tempfile.TemporaryDirectory() as temp:
                outputs, c, api = self.fixture(temp)
                selected = c
                if fault == "foreign":
                    selected = original_custody()
                elif fault in ("pid", "birth"):
                    c.exit_proof["witnesses"][0]["pid" if fault == "pid" else "creation_100ns"] += 1
                elif fault == "status":
                    c.owner.poll.return_value = 1
                else:
                    c.custody_refusal = "query refused"
                with self.assertRaises(Refusal):
                    outputs.seal(selected)
                self.assertEqual(api.read_calls, 0)
                outputs.close()

    def test_file_directory_replacement_and_writer_refuse_before_read(self):
        for fault in ("directory", "stdout.log", "writer"):
            with tempfile.TemporaryDirectory() as temp:
                outputs, c, api = self.fixture(temp)
                if fault == "writer":
                    api.writer = "stderr.log"
                else:
                    path = outputs.path if fault == "directory" else outputs.path / fault
                    api.nodes[str(path)]["index"] += 100
                with self.assertRaises(Refusal):
                    outputs.seal(c)
                self.assertEqual(api.read_calls, 0)
                with self.assertRaises(Refusal):
                    outputs.seal(c)
                outputs.close()

    def test_close_failure_prevents_payload_success_and_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            outputs, c, api = self.fixture(temp)
            # Existing handles: parent/reservation/directory/three live files.
            # seal first opens result, then three directories and four file locks.
            api.bad_close = api.next_id + 8
            with self.assertRaisesRegex(Refusal, "output close"):
                outputs.seal(c)
            self.assertIsNotNone(outputs.first_refusal)
            outputs.close()

    def test_owner_exit_during_attach_and_open_error_are_sticky(self):
        for fault in ("exit", "open"):
            with tempfile.TemporaryDirectory() as temp:
                api = MemoryFiles()
                outputs = AttemptOutputs("fixed-attempt", temp, temp, None, api)
                c = original_custody()
                if fault == "exit":
                    c.owner.poll.side_effect = [None, 0]
                    with self.assertRaisesRegex(Refusal, "during output attachment"):
                        outputs.attach(c)
                    c.owner.poll.side_effect = None
                else:
                    with patch.object(api, "open", side_effect=Refusal("open refused")):
                        with self.assertRaisesRegex(Refusal, "open refused"):
                            outputs.attach(c)
                first = outputs.first_refusal
                with self.assertRaises(Refusal):
                    outputs.attach(c)
                self.assertEqual(outputs.first_refusal, first)
                c.owner.poll.return_value = 0
                outputs.close()


class WindowsApiModelTests(unittest.TestCase):
    def api(self):
        api = object.__new__(WindowsFiles)
        api.dll = SimpleNamespace(CreateFileW=Mock(return_value=17), CloseHandle=Mock(return_value=True))
        api.info = Mock(return_value=dict(volume=1, index=2, attributes=0, links=1, size=0, written=3))
        return api

    def test_exact_noninheritable_no_write_no_delete_open_and_type_guards(self):
        api = self.api()
        self.assertEqual(api.open("fixed.log", sealed=True), 17)
        args = api.dll.CreateFileW.call_args.args
        self.assertEqual(args[1:6], (0x80000000, 1, None, 3, 0x00200000))
        for attributes, links in ((0x400, 1), (0x10, 1), (0, 2)):
            api = self.api()
            api.info.return_value.update(attributes=attributes, links=links)
            with self.assertRaises(Refusal):
                api.open("fixed.log")
            api.dll.CloseHandle.assert_called_once_with(17)

    def test_invalid_handle_and_close_errors_refuse(self):
        with patch.object(ctypes, "get_last_error", return_value=5, create=True):
            api = self.api()
            api.dll.CreateFileW.return_value = ctypes.c_void_p(-1).value
            with self.assertRaises(Refusal):
                api.open("fixed.log")
            api.dll.CloseHandle.return_value = False
            with self.assertRaises(Refusal):
                api.close(17)

    def test_empty_read_short_read_and_metadata_drift(self):
        api = self.api()
        self.assertEqual(api.read(17, 0), b"")
        api.info.return_value["size"] = 1
        api.dll.ReadFile = Mock(return_value=True)  # zero bytes, not a silent EOF
        with self.assertRaisesRegex(Refusal, "short original"):
            api.read(17, 1)
        api = self.api()
        before = api.info.return_value.copy()
        api.info.side_effect = [before, dict(before, written=4)]
        with self.assertRaisesRegex(Refusal, "changed during read"):
            api.read(17, 1)


@unittest.skipUnless(os.name == "nt", "Windows share/identity semantics")
class WindowsFileControls(unittest.TestCase):
    def test_real_writer_and_delete_sharing_preserve_original_inode(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "original.log"
            api = WindowsFiles()
            with path.open("xb") as writer:
                writer.write(b"first"); writer.flush()
                held = api.open(path)
                try:
                    with self.assertRaises(Refusal):
                        api.open(path, sealed=True)
                    with self.assertRaises(OSError):
                        path.rename(Path(temp) / "replacement.log")
                finally:
                    api.close(held)
            sealed = api.open(path, sealed=True)
            try:
                self.assertEqual(api.read(sealed, 5), b"first")
                with self.assertRaises(OSError):
                    path.open("ab")
            finally:
                api.close(sealed)

    def test_real_read_limit_directory_and_hardlink_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "original.log"
            path.write_bytes(b"1234")
            api = WindowsFiles()
            held = api.open(path, sealed=True)
            try:
                with self.assertRaisesRegex(Refusal, "fixed read limit"):
                    api.read(held, 3)
            finally:
                api.close(held)
            with self.assertRaises(Refusal):
                api.open(Path(temp))
            os.link(path, Path(temp) / "alias.log")
            with self.assertRaisesRegex(Refusal, "hard-link"):
                api.open(path)


if __name__ == "__main__":
    unittest.main()
