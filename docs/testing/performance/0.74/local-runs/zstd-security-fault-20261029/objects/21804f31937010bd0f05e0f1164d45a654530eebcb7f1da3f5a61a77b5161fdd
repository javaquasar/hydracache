import copy
import unittest

import analyze as a


def fixture():
    steps = tuple('S' + str(i) for i in range(8))
    stages = tuple('E' + str(i) for i in range(9))
    marks = [dict(name='outputs-enter', wall_ns=0, process_cpu_ns=0)]
    for i in range(9):
        marks += [dict(name='st-effect', wall_ns=i * 10 + 1, process_cpu_ns=i * 10 + 1),
                  dict(name='st-effected', wall_ns=i * 10 + 2, process_cpu_ns=i * 10 + 2)]
    marks += [dict(name=n, wall_ns=t, process_cpu_ns=t) for n, t in
              (('outputs-starting', 90), ('outputs-pipes', 100), ('outputs-sinks', 110))]
    return dict(phase='outputs', returned=True, instrumented=True, guard_clock_added=False,
        deadline_changed=False, healthy_gate=False, includes_child_cpu=False,
        overlapping_spans=True, wall_ns=120, boundaries=marks,
        round_steps=[dict(step=i, calls=46, completed=46, wall_ns=5,
                          guard_clock_added=False, authentication_calls=1) for i in range(8)]), steps, stages


class Tests(unittest.TestCase):
    def test_stage_pairing_and_disjoint_steps_without_overlap_promotion(self):
        result = a.review(*fixture())
        self.assertEqual(result['storage_effect_wall_ns'], 9)
        self.assertEqual(result['validation_step_wall_ns'], 40)
        self.assertFalse(result['syscall_only_cost'])
        self.assertFalse(result['new_server_run'])
        self.assertEqual([s['wall_ns'] for s in result['output_sections']], [90, 10, 10])

    def test_missing_unpaired_reordered_or_incomplete_observations_refuse(self):
        for case in ('missing', 'nested', 'backward', 'round', 'incomplete', 'guard', 'duplicate', 'sum'):
            phase, steps, stages = copy.deepcopy(fixture())
            if case == 'missing': phase['boundaries'].pop(1)
            if case == 'nested': phase['boundaries'][2]['name'] = 'st-effect'
            if case == 'backward': phase['boundaries'][2]['wall_ns'] = 0
            if case == 'round': phase['round_steps'][0]['step'] = 1
            if case == 'incomplete': phase['round_steps'][0]['completed'] = 45
            if case == 'guard': phase['deadline_changed'] = True
            if case == 'duplicate': phase['boundaries'].append(phase['boundaries'][-1])
            if case == 'sum': phase['round_steps'][0]['wall_ns'] = 121
            with self.subTest(case=case), self.assertRaises(ValueError):
                a.review(phase, steps, stages)

    def test_missing_cpu_stays_unknown_and_enum_source_is_closed(self):
        phase, steps, stages = fixture()
        phase['boundaries'][1]['process_cpu_ns'] = None
        self.assertIsNone(a.review(phase, steps, stages)['storage_effect_process_cpu_ns'])
        self.assertEqual(a.enum('enum Example { A, B, }', 'Example'), ('A', 'B'))
        with self.assertRaises(ValueError): a.enum('enum Example { A } enum Example { B }', 'Example')


if __name__ == '__main__':
    unittest.main()
