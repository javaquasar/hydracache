"""Synthetic protocol controls, not numerical measurements."""
import copy
import json
import unittest

import pipe_review as review


def fixture():
    rows = []
    for worker in range(4):
        for block in range(6):
            order = ('control', 'profile', 'profile', 'control') if block % 2 == 0 else ('profile', 'control', 'control', 'profile')
            for slot, variant in enumerate(order):
                start = dict(pid=1+worker*24+block*4+slot, created_100ns=1, exited_100ns=0, kernel_ns=0, user_ns=0)
                end = dict(start, exited_100ns=10, kernel_ns=1, user_ns=1)
                phase = dict(spawn_ns=1, handle_observation_ns=1, wait_and_collect_ns=1,
                             terminal_observation_ns=1, child_start=start, child_end=end)
                rows.append(dict(worker=worker, block=block, slot=slot, variant=variant, setup_ns=1,
                                 wall_ns=8, phases=None if variant == 'control' else phase,
                                 stdout_bytes=23589, stderr_bytes=0, code=0))
    done = dict(samples=96, workers=4, blocks=6, wall_ns=100, parent_cpu_ns=1, spec=review.SPEC,
                body_sha256=review.BODY, product_candidate=False, installed_helper_changed=False, admission=False)
    return rows, done


def raw(rows, done):
    return ('\n'.join('HC074_PIPE '+json.dumps(v) for v in rows)+'\nHC074_PIPE_DONE '+json.dumps(done)).encode()


class PipeReview(unittest.TestCase):
    def test_complete(self):
        value = review.read(raw(*fixture()))
        self.assertEqual(value['samples'], 96)
        self.assertEqual(value['observed_children'], 48)
        self.assertEqual(value['medians_ns']['child_cpu_ns'], 2)

    def test_missing_duplicate_or_reordered_variant(self):
        rows, done = fixture()
        for bad in (rows[:-1], rows[:-1]+[rows[0]]):
            with self.assertRaises(ValueError): review.read(raw(bad, done))
        rows[0]['variant'] = 'profile'
        with self.assertRaises(ValueError): review.read(raw(rows, done))

    def test_identity_time_phase_and_schema_refusals(self):
        rows, done = fixture()
        changes = [('child_end', 'pid', 999), ('child_end', 'created_100ns', 2),
                   ('child_end', 'exited_100ns', 0), ('child_start', 'kernel_ns', 9)]
        for scope, name, value in changes:
            bad = copy.deepcopy(rows); bad[1]['phases'][scope][name] = value
            with self.assertRaises(ValueError): review.read(raw(bad, done))
        for name, value in [('wall_ns', True), ('code', 7), ('stdout_bytes', 1), ('unknown', 0)]:
            bad = copy.deepcopy(rows); bad[0][name] = value
            with self.assertRaises(ValueError): review.read(raw(bad, done))
        bad = copy.deepcopy(rows); bad[1]['phases']['wait_and_collect_ns'] = 20
        with self.assertRaises(ValueError): review.read(raw(bad, done))

    def test_terminal_and_unknown_marker(self):
        rows, done = fixture()
        for key, value in [('admission', True), ('body_sha256', 'observed'), ('samples', 95)]:
            bad = dict(done); bad[key] = value
            with self.assertRaises(ValueError): review.read(raw(rows, bad))
        with self.assertRaises(ValueError): review.read(raw(rows, done)+b'\nunsupported HC074_PIPE')
        with self.assertRaises(ValueError): review.read(raw(rows, done)+b'\nHC074_PIPE '+json.dumps(rows[0]).encode())
        with self.assertRaises(ValueError): review.read(b'HC074_PIPE {"worker":0,"worker":1}')
