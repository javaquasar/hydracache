"""Offline/fake-tracefs and owned-process watchdog tests, never live probes."""
import json
import copy
import os
from pathlib import Path
import select
import signal
import struct
import tempfile
import time
import unittest
from unittest.mock import patch, MagicMock

import btf
import capture as cap
import analyze
import run

GROUP = 'hc074clock_' + 'a' * 32


class Plan(unittest.TestCase):
    def test_buffered_write_and_close_errors_keep_exact_path(self):
        for method in ('write', '__exit__'):
            stream = MagicMock()
            stream.__enter__.return_value = stream
            getattr(stream, method).side_effect = OSError(22, 'Invalid argument')
            path = Path('/own/fake/control')
            with patch.object(Path, 'open', return_value=stream), self.assertRaises(OSError) as caught:
                cap.write(path, '1')
            self.assertEqual(caught.exception.errno, 22)
            self.assertEqual(caught.exception.filename, str(path))

    def test_pid_stat_comm_with_spaces_and_parentheses(self):
        self.assertEqual(cap.pid1_start('1 (a ) comm) S ' + '0 ' * 18 + '10939 0'), 10939)

    def test_only_the_owned_group_and_expected_write_paths(self):
        planned = cap.definitions(GROUP)
        self.assertEqual(len(planned), 7)
        self.assertTrue(all(cap.key(s).startswith(GROUP + '/') for s in planned))
        self.assertIn('modes=+0(%di):u32', planned[0])
        for value in ('', '../other', 'hc074clock_' + 'z' * 32):
            with self.assertRaises(ValueError):
                cap.definitions(value)

    def test_btf_malformed_and_truncated_refuse(self):
        for value in (b'', b'wrong header' * 2, struct.pack('<HBBIIIII', 0xEB9F, 1, 0, 24, 0, 20, 20, 1)):
            with self.assertRaises((ValueError, struct.error)):
                btf.types(value)

    @unittest.skipUnless(Path('/sys/kernel/btf/vmlinux').exists(), 'Read-only kernel ABI control')
    def test_btf_real_layout_and_corrupt_member_or_prototype_refuse(self):
        raw = Path('/sys/kernel/btf/vmlinux').read_bytes()
        self.assertEqual(btf.layouts(raw)['__kernel_timex']['tick'], 88)
        decoded = btf.types(raw)
        for mutation in ('offset', 'bits', 'width', 'prototype'):
            altered = copy.deepcopy(decoded)
            timex = next(i for i in altered if i and i['kind'] == 4 and i['name'] == '__kernel_timex')
            if mutation == 'offset':
                typ, _ = timex['members']['tick']
                timex['members']['tick'] = (typ, 80)
            elif mutation == 'bits':
                timex['bitfields']['tick'] = (1, 0)
            elif mutation == 'width':
                typ = timex['members']['tick'][0]
                altered[typ]['size'] = 4
            else:
                func = next(i for i in altered if i and i['kind'] == 12 and i['name'] == 'do_adjtimex')
                altered[func['size']]['args'][0] = timex['members']['tick'][0]
            with patch.object(btf, 'types', return_value=altered), self.assertRaises(ValueError):
                btf.layouts(raw)

    def test_no_truncate_of_global_probe_registry_or_root_control_writes(self):
        source = Path(cap.__file__).read_text()
        self.assertIn("write(TRACE / 'kprobe_events', definition, append=True)", source)
        self.assertIn("write(root / 'kprobe_events', '-:' + name, append=True)", source)
        for field in ('current_tracer', 'trace_clock', 'tracing_on', 'trace_options'):
            self.assertNotIn(f"write(TRACE / '{field}'", source)
        for forbidden in ('clock_settime(', 'clock_adjtime(', 'settimeofday(', 'systemctl', 'native_fixture'):
            self.assertNotIn(forbidden, source)


class Cleanup(unittest.TestCase):
    def fixture(self, root, changed=False):
        (root / 'instances' / GROUP).mkdir(parents=True)
        own = cap.definitions(GROUP)[0]
        actual = own.replace('freq=+16', 'freq=+24') if changed else own
        foreign = 'p:other/unrelated unrelated_function'
        (root / 'kprobe_events').write_text(actual + '\n' + foreign + '\n')
        return own, foreign

    def test_partial_and_repeated_cleanup_never_remove_foreign_probe(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            own, foreign = self.fixture(root)
            commands = []
            def fake_write(path, value, append=False):
                commands.append((path, value, append))
                if path == root / 'kprobe_events':
                    self.assertEqual(value, '-:' + cap.key(own))
                    self.assertTrue(append)
                    path.write_text(foreign + '\n')
            with patch.object(cap, 'write', side_effect=fake_write):
                first = cap.cleanup(GROUP, cap.definitions(GROUP), root)
                second = cap.cleanup(GROUP, cap.definitions(GROUP), root)
            self.assertTrue(first['ok'])
            self.assertTrue(second['ok'])
            self.assertEqual((root / 'kprobe_events').read_text(), foreign + '\n')
            self.assertFalse(any(value == '-:other/unrelated' for _, value, _ in commands))

    def test_changed_definition_is_not_deleted(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.fixture(root, changed=True)
            with patch.object(cap, 'write') as write:
                result = cap.cleanup(GROUP, cap.definitions(GROUP), root)
            self.assertFalse(result['ok'])
            self.assertEqual(len(result['remaining_owned_probes']), 1)
            self.assertFalse(any(args[1].startswith('-:') for args, _ in write.call_args_list))


@unittest.skipUnless(hasattr(os, 'fork') and hasattr(os, 'pidfd_open'), 'Linux owned-process controls')
class Guardian(unittest.TestCase):
    def wait_file(self, path, complete):
        try:
            self.assertTrue(select.select([complete], [], [], 4)[0], 'Bounded guardian receipt missing')
            self.assertEqual(os.read(complete, 1), b'D', 'Receipt publication did not complete')
        finally:
            os.close(complete)
        self.assertTrue(path.exists(), 'Completed guardian receipt missing')
        return json.loads(path.read_text())

    def start(self, folder, mode):
        done_r, done_w = os.pipe()
        pid = os.fork()
        if pid == 0:
            os.close(done_r)
            cap.OUT = Path(folder)
            owner = os.pidfd_open(os.getpid())
            r, w = os.pipe()
            rr, rw = os.pipe()
            child = os.fork()
            if child == 0:
                os.close(w); os.close(rr)
                publish = cap.publish
                def complete(path, value):
                    publish(path, value)  # Signal only after the real write/flush/fsync.
                    os.write(done_w, b'D')
                with patch.object(cap, 'cleanup', return_value={'ok': True, 'instance_absent': True}), \
                     patch.object(cap, 'publish', side_effect=complete):
                    cap.guardian(owner, r, rw, GROUP, cap.definitions(GROUP),
                                 cap.raw_ns() + (50_000_000 if mode == 'deadline' else 3_000_000_000))
            os.close(r); os.close(rw)
            os.close(done_w)
            if not select.select([rr], [], [], 2)[0] or os.read(rr, 1) != b'R':
                os._exit(3)
            if mode == 'normal':
                os.write(w, b'D')
            elif mode == 'owner-kill':
                os.kill(os.getpid(), signal.SIGKILL)
            os.waitpid(child, 0)
            os._exit(0)
        os.close(done_w)
        return pid, done_r

    def test_normal_completion(self):
        with tempfile.TemporaryDirectory() as folder:
            owner, complete = self.start(folder, 'normal')
            self.assertEqual(os.waitpid(owner, 0)[1], 0)
            self.assertEqual(self.wait_file(Path(folder) / 'guardian.json', complete)['reason'], 'normal_completion')

    def test_owner_sigkill_with_external_harness(self):
        with tempfile.TemporaryDirectory() as folder:
            owner, complete = self.start(folder, 'owner-kill')
            self.assertTrue(os.WIFSIGNALED(os.waitpid(owner, 0)[1]))
            self.assertEqual(self.wait_file(Path(folder) / 'guardian.json', complete)['reason'], 'owner_exit_or_control_close')

    def test_hard_deadline_without_controller(self):
        with tempfile.TemporaryDirectory() as folder:
            owner, complete = self.start(folder, 'deadline')
            self.assertEqual(os.waitpid(owner, 0)[1], 0)
            self.assertEqual(self.wait_file(Path(folder) / 'guardian.json', complete)['reason'], 'hard_deadline')

    def test_visible_empty_receipt_is_not_read_before_completion(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'guardian.json'
            path.write_text('')  # File visibility alone is deliberately not completion.
            done_r, done_w = os.pipe()
            child = os.fork()
            if child == 0:
                os.close(done_r)
                time.sleep(0.02)
                cap.publish(path.with_name('complete.json'), {'complete': True})
                os.replace(path.with_name('complete.json'), path)
                os.write(done_w, b'D')
                os._exit(0)
            os.close(done_w)
            try:
                self.assertEqual(self.wait_file(path, done_r), {'complete': True})
            finally:
                self.assertEqual(os.waitpid(child, 0)[1], 0)


class Offline(unittest.TestCase):
    def test_entries_returns_stack_and_no_false_query_write(self):
        raw = '''worker-21 [003] .... 100.100000: settime: (do_settimeofday64+0x0) sec=1790000000 ns=200
worker-21 [003] .... 100.100001: <stack trace>
 => hv_set_host_time+0x13/0x50
 => process_one_work+0x1/0x20
helper-22 [002] .... 100.100002: adjust_return: (x <- do_adjtimex) retval=0
worker-21 [003] .... 100.100003: settime_return: (x <- do_settimeofday64) retval=0
'''
        records = analyze.parse(raw)
        calls, orphan, incomplete = analyze.pair(records)
        self.assertEqual(calls[0]['entry']['stack'][0], 'hv_set_host_time+0x13/0x50')
        self.assertTrue(calls[0]['successful'])
        self.assertEqual(calls[0]['entry']['raw_ns'], 100100000000)
        self.assertEqual(orphan, {'adjust': 1})
        self.assertFalse(incomplete)
        records[-1]['fields']['retval'] = -1
        self.assertFalse(analyze.pair(records)[0][0]['successful'])

    def test_missing_argument_and_unknown_event_refuse(self):
        for payload in ('settime: (func) sec=10 ns=(fault)', 'other: value=0'):
            with self.assertRaises(ValueError):
                analyze.parse('worker-21 [003] .... 100.100000: ' + payload)

    def test_incomplete_entry_and_wrong_pid_never_imply_success(self):
        records = analyze.parse('worker-21 [003] .... 100.100000: offset: (func) sec=-1 ns=5\n'
                                'worker-22 [003] .... 100.100001: offset_return: (func) retval=0')
        calls, orphan, incomplete = analyze.pair(records)
        self.assertFalse(calls)
        self.assertEqual(orphan, {'offset': 1})
        self.assertEqual(len(incomplete), 1)

    def test_loss_missing_counter_and_kprobe_misses_refuse(self):
        stats = {'cpu0': 'overrun: 0\ncommit overrun: 0\ndropped events: 0\n'}
        profile = '\n'.join(e + ' 1 0' for e in sorted(analyze.EVENTS))
        self.assertEqual(len(analyze.loss_check(stats, profile)), 7)
        for invalid, p in (({}, profile), ({'cpu0': 'overrun: 0'}, profile),
                           ({'cpu0': stats['cpu0'].replace('overrun: 0', 'overrun: 1')}, profile),
                           (stats, profile.replace('adjust 1 0', 'adjust 1 1'))):
            with self.assertRaises(ValueError):
                analyze.loss_check(invalid, p)

    def test_command_first_timeout_exclusive_logs_no_retry(self):
        import subprocess
        with tempfile.TemporaryDirectory() as folder, patch.object(run, 'OUT', Path(folder)):
            with patch.object(run.subprocess, 'run', side_effect=subprocess.TimeoutExpired(['owned'], 1)) as call:
                with self.assertRaises(RuntimeError):
                    run.command('test', ['owned'], 1)
                self.assertEqual(call.call_count, 1)
                self.assertEqual(json.loads((Path(folder) / 'test.json').read_text())['exit'], 124)
                with self.assertRaises(FileExistsError):
                    run.command('test', ['owned'], 1)
                self.assertEqual(call.call_count, 1)

    def test_archive_exact_bytes_and_tampering_refusal(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            out, archive = root / 'out', root / 'archive'
            out.mkdir()
            src = root / 'source.py'
            src.write_text('unchanged\n')
            proof = {'source.py': run.digest(src.read_bytes())}
            run.put(out / 'intent.json', {'sources': proof, 'frozen_inputs': run.FROZEN})
            (out / 'trace.txt').write_text('first outcome\n')
            with patch.object(run, 'ROOT', root), patch.object(run, 'OUT', out), patch.object(run, 'ARCHIVE', archive), patch.object(run, 'sources', return_value=proof):
                run.retain()
                run.verify()
                entry = json.loads((archive / 'index.json').read_text())['files']['observation/trace.txt']
                (archive / 'objects' / entry['sha256']).write_text('changed\n')
                with self.assertRaises(ValueError):
                    run.verify()


if __name__ == '__main__':
    unittest.main()
