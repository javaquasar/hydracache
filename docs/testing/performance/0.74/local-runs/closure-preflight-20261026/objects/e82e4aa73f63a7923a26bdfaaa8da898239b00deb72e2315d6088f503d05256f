"""Bounded Linux CI fixtures; not original-start or hostile-writer proof."""
import importlib.util
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import time
import unittest

HERE = Path(__file__).resolve()


def fixture(mode, folder):
    if mode == 'writer':
        (folder/'writer.json').write_text(json.dumps(dict(pid=os.getpid())), encoding='utf-8')
        until = time.monotonic()+6
        with (folder/'writes.bin').open('ab', buffering=0) as stream:
            while time.monotonic() < until:
                stream.write(b'x')
                time.sleep(.02)
        return
    subprocess.Popen([sys.executable, str(HERE), 'fixture', 'writer', str(folder)],
                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                     start_new_session=(mode == 'detached'))
    until = time.monotonic()+4
    while not (folder/'release').exists() and time.monotonic() < until:
        time.sleep(.01)
    raise SystemExit(23 if mode == 'failure' else 0)


@unittest.skipUnless(sys.platform.startswith('linux'), 'Linux-only real process controls')
class Native(unittest.TestCase):
    def wrapper(self):
        return Path(os.environ.get('HC074_WATCHDOG_ENTRY', str(HERE.with_name('run-with-heartbeat.py'))))

    def command(self, folder, *child):
        return [sys.executable, str(self.wrapper()), '--timeout-seconds', '2', '--heartbeat-seconds', '.1',
                '--status-json', str(folder/'receipt.json'), '--log-file', str(folder/'child.log'), '--', *child]

    def orphan(self, failed, detached=False):
        with tempfile.TemporaryDirectory(prefix='hc074-ci-group-') as directory:
            folder, writer = Path(directory), None
            process = subprocess.Popen(self.command(folder, sys.executable, str(HERE), 'fixture',
                'detached' if detached else ('failure' if failed else 'root'), directory),
                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                until = time.monotonic()+2
                while not (folder/'writer.json').exists():
                    if time.monotonic() >= until:
                        self.fail('owned writer did not become ready')
                    time.sleep(.01)
                pid = json.loads((folder/'writer.json').read_text())['pid']
                writer = os.pidfd_open(pid)
                self.assertEqual(select.select([writer], [], [], 0)[0], [])
                (folder/'release').touch(exist_ok=False)
                stdout, stderr = process.communicate(timeout=8)
                receipt = json.loads((folder/'receipt.json').read_text())
                self.assertEqual(process.returncode, 23 if failed else 124, (stdout, stderr, receipt))
                self.assertEqual(receipt['classification'], 'product-failure' if failed else 'timeout')
                self.assertEqual(select.select([writer], [], [], 1)[0], [writer], 'original writer remains live')
                size = (folder/'writes.bin').stat().st_size
                time.sleep(.1)
                self.assertEqual((folder/'writes.bin').stat().st_size, size)
            finally:
                if writer is not None:
                    if not select.select([writer], [], [], 0)[0]:
                        signal.pidfd_send_signal(writer, signal.SIGKILL)
                    self.assertEqual(select.select([writer], [], [], 2)[0], [writer])
                    os.close(writer)
                if process.poll() is None:
                    process.kill()  # Original unreaped owned harness child only.
                process.communicate(timeout=5)

    def test_zero_exit_leader_with_live_writer_is_timeout(self):
        self.orphan(False)

    def test_failed_leader_still_stops_original_writer(self):
        self.orphan(True)

    def test_new_session_writer_is_adopted_and_stopped(self):
        self.orphan(False, detached=True)

    def test_plain_success_and_failure_keep_classification(self):
        for code in (0, 23):
            with tempfile.TemporaryDirectory(prefix='hc074-ci-group-') as directory:
                folder = Path(directory)
                result = subprocess.run(self.command(folder, sys.executable, '-c', f'raise SystemExit({code})'),
                    capture_output=True, timeout=8)
                self.assertEqual(result.returncode, code, result.stderr)
                receipt = json.loads((folder/'receipt.json').read_text())
                self.assertEqual(receipt['classification'], 'success' if code == 0 else 'product-failure')

    def test_missing_program_remains_tool_unavailable(self):
        with tempfile.TemporaryDirectory(prefix='hc074-ci-group-') as directory:
            folder = Path(directory)
            result = subprocess.run(self.command(folder, str(folder/'missing-program')), capture_output=True, timeout=8)
            self.assertEqual(result.returncode, 127, result.stderr)
            self.assertEqual(json.loads((folder/'receipt.json').read_text())['classification'], 'tool-unavailable')


if __name__ == '__main__':
    if len(sys.argv) == 4 and sys.argv[1] == 'fixture':
        fixture(sys.argv[2], Path(sys.argv[3]))
    else:
        unittest.main()
