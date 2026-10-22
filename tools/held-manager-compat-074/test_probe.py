"""Synthetic offline controls. No WSL, bus or service command is executed."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import probe as p

UNIT = p.PREFIX + '74' * 16 + '.service'


def reply(kind, value):
    return dict(exit=0, stdout=json.dumps(dict(type=kind, data=value if kind == 'v' else [value])), stderr='')


def missing():
    return dict(exit=1, stdout='', stderr=f'Call failed: Unit {UNIT} not loaded.\n')


class Bus:
    def __init__(self):
        self.rows = [reply('s', ':1.74'), reply('u', 1), reply('u', 0),
            reply('v', [dict(type='s', data='255.4-1ubuntu8.18')]), missing(),
            dict(exit=1, stdout='', stderr='Call failed: Cannot set property ExtraFileDescriptors, or unknown property.\n'),
            missing(), reply('s', ':1.74'), reply('u', 1), reply('u', 0)]
        self.calls = []

    def call(self, *args):
        self.calls.append(args)
        return self.rows[len(self.calls) - 1]


class Controls(unittest.TestCase):
    def test_first_actual_readonly_reply_uses_a_single_argument_array(self):
        captured = dict(exit=0, stdout='{"type":"s","data":[":1.2"]}\n', stderr='')
        self.assertEqual(p.scalar(captured, 's'), ':1.2')
        for data in (':1.2', [], [':1.2', ':1.3'], [74]):
            with self.assertRaises(ValueError):
                p.scalar(dict(exit=0, stdout=json.dumps(dict(type='s', data=data)), stderr=''), 's')

    def test_negative_refusal_has_no_original_admission_and_only_one_start(self):
        bus = Bus()
        result = p.check(bus, UNIT)
        self.assertFalse(result['original_delivery'])
        self.assertFalse(result['writers_cessation'])
        self.assertFalse(result['admission_allowed'])
        self.assertFalse(result['fallback'])
        starts = [v for v in bus.calls if v[3] == 'StartTransientUnit']
        self.assertEqual(len(starts), 1)
        self.assertEqual(starts[0][0], ':1.74')
        self.assertEqual(starts[0][5], UNIT)
        self.assertIn('a(hs)', starts[0])
        self.assertEqual(starts[0][-5:], ('/usr/bin/false', '1', '/usr/bin/false', 'false', '0'))
        self.assertTrue(all(v[3] in ('GetNameOwner', 'GetConnectionUnixProcessID', 'GetConnectionUnixUser', 'Get', 'GetUnit', 'StartTransientUnit') for v in bus.calls))

    def test_owner_or_version_or_collision_refuses_before_any_start(self):
        for index, value in ((0, reply('s', 'org.freedesktop.systemd1')), (1, reply('u', 74)),
                (2, reply('u', 1000)), (3, reply('v', [dict(type='s', data='257')])),
                (4, reply('o', '/unit/already_exists'))):
            with self.subTest(index=index):
                bus = Bus(); bus.rows[index] = value
                with self.assertRaises(ValueError): p.check(bus, UNIT)
                self.assertFalse(any(v[3] == 'StartTransientUnit' for v in bus.calls))

    def test_unsafe_unit_never_queries_or_mutates(self):
        for name in ('sshd.service', 'system.slice', UNIT + '/other', '../' + UNIT, 'hc074-fdcompat-X.service'):
            bus = Bus()
            with self.assertRaises(ValueError): p.check(bus, name)
            self.assertEqual(bus.calls, [])

    def test_no_failure_or_ack_may_be_retried_into_a_compatibility_pass(self):
        for value in (reply('o', '/org/freedesktop/systemd1/job/74'),
                dict(exit=1, stdout='', stderr='Call failed: Access denied\n'),
                dict(exit=1, stdout='', stderr='Call failed: Message recipient disconnected\n'),
                dict(exit=1, stdout='', stderr='Call failed: Invalid signature\n')):
            bus = Bus(); bus.rows[5] = value
            with self.assertRaises(ValueError): p.check(bus, UNIT)
            self.assertEqual(sum(v[3] == 'StartTransientUnit' for v in bus.calls), 1)

    def test_uncertain_unit_is_not_blindly_stopped_or_reset(self):
        bus = Bus(); bus.rows[6] = reply('o', '/org/freedesktop/systemd1/unit/uncertain')
        with self.assertRaisesRegex(ValueError, 'Uncertain private unit'): p.check(bus, UNIT)
        self.assertFalse(any(v[3] in ('StopUnit', 'ResetFailedUnit') for v in bus.calls))

    def test_manager_churn_refuses_after_the_single_attempt(self):
        bus = Bus(); bus.rows[7] = reply('s', ':1.75')
        with self.assertRaisesRegex(ValueError, 'Manager changed'): p.check(bus, UNIT)
        self.assertEqual(sum(v[3] == 'StartTransientUnit' for v in bus.calls), 1)

    def test_strict_reply_schema_and_version_never_supply_authority(self):
        for raw in ('255', '255.4-1ubuntu8.18', '257', '257.13'):
            self.assertGreaterEqual(p.version(raw), 255)
        for raw in ('255\n', '0255', '255/evil', 'v257', '', '255.' + 'x' * 100):
            with self.assertRaises(ValueError): p.version(raw)
        for data in (dict(type='s', data='255'), [], [dict(type='s', data='255')] * 2,
                [dict(type='u', data=255)]):
            with self.assertRaises(ValueError): p.manager_version(reply('v', data))
        with self.assertRaises(ValueError): p.scalar(reply('u', True), 'u')
        self.assertFalse(p.absent(dict(exit=1, stdout='', stderr='Call failed: Access denied'), UNIT))

    def test_existing_attempt_is_not_resumed_and_unsafe_attempt_has_no_context_access(self):
        with patch.object(p, 'context') as context:
            with self.assertRaises(ValueError): p.run('../other')
            context.assert_not_called()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'target/held-manager-compat-existing').mkdir(parents=True)
            with patch.object(p, 'ROOT', root), patch.object(p, 'context', return_value={}), patch.object(p, 'Bus') as bus:
                with self.assertRaises(ValueError): p.run('existing')
                bus.assert_not_called()

    def test_receipts_and_timeout_are_retained_without_second_subprocess(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(p.subprocess, 'run', side_effect=p.subprocess.TimeoutExpired('synthetic', 1)) as child:
                with self.assertRaises(p.subprocess.TimeoutExpired):
                    p.Bus(root).call(':1.74', p.MANAGER_PATH, p.MANAGER + '.Manager', 'GetUnit', 's', UNIT)
                self.assertEqual(child.call_count, 1)
            value = json.loads((root / '000-command.json').read_text())
            self.assertTrue(value['failure'].startswith('TimeoutExpired:'))
            self.assertIn('--auto-start=no', value['argv'])
            self.assertIn('--timeout=250ms', value['argv'])
            with self.assertRaises(FileExistsError): p.publish(root / '000-command.json', {})


if __name__ == '__main__': unittest.main(verbosity=2)
