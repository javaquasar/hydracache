import copy
import json
import unittest

import helper_review as reader


def fixture():
    def row(ident, parent, op, start, end, args=()):
        return dict(id=ident, parent=parent, operation=op, start_ns=start, end_ns=end,
                    elapsed_ns=end-start, arguments=list(args), returned=True, unwinding=False,
                    output_bytes=11, stderr_bytes=0)
    return dict(schema='hc074-helper-trace-v1', case='fixture', thread='ThreadId(1)', valid=True,
                started=4, nested_intervals=True, product=False, package_replay=False,
                records=[row(2, 1, 'output', 10, 50), row(3, 1, 'decode', 60, 70),
                         row(1, 0, 'git', 0, 80, ('show', 'pinned:fixture')),
                         row(4, 0, 'read', 90, 110, ('fixture',))])


def encode(value):
    return ('HC074_HELPER_TRACE ' + json.dumps(value) + '\n').encode()


class HelperReaderControls(unittest.TestCase):
    def test_losing_an_entire_case_trace_cannot_hide_behind_other_complete_cases(self):
        first, second = fixture(), fixture()
        second.update(case='second', thread='ThreadId(2)')
        expected = {'fixture', 'second'}
        self.assertEqual(len(reader.traces(encode(first)+encode(second), expected, required_cases=expected)['frames']), 2)
        with self.assertRaises(ValueError):
            reader.traces(encode(first), expected, required_cases=expected)

    def test_nested_count_bytes_and_times_are_not_cpu_or_disjoint_target_phases(self):
        result = reader.traces(encode(fixture()), {'fixture'})
        self.assertEqual(result['counts'], dict(git=1, output=1, decode=1, read=1))
        self.assertEqual(result['shows'], 1)
        self.assertEqual(result['groups']['fixture/output']['cumulative_wall_ns'], 40)
        self.assertTrue(result['cumulative_wall_not_target_cpu'])
        self.assertFalse(result['observer_overhead_subtracted'])

    def test_damaged_graph_times_types_bytes_completion_and_scope_refuse(self):
        for mode in range(16):
            frame = copy.deepcopy(fixture())
            if mode == 0: frame['valid'] = False
            elif mode == 1: frame['started'] = True
            elif mode == 2: frame['records'].pop()
            elif mode == 3: frame['records'][0]['id'] = True
            elif mode == 4: frame['records'][0]['elapsed_ns'] = 41
            elif mode == 5: frame['records'][0]['start_ns'] = -1
            elif mode == 6: frame['records'][0]['returned'] = False
            elif mode == 7: frame['records'][0]['unwinding'] = True
            elif mode == 8: frame['records'][0]['output_bytes'] = True
            elif mode == 9: frame['records'][0]['arguments'] = ['extra']
            elif mode == 10: frame['records'][1]['parent'] = 2
            elif mode == 11: frame['records'][1]['operation'] = 'output'
            elif mode == 12: frame['records'][0], frame['records'][1] = frame['records'][1], frame['records'][0]
            elif mode == 13: frame['records'][1]['end_ns'] = 100; frame['records'][1]['elapsed_ns'] = 40
            elif mode == 14: frame['records'][2]['arguments'] = ['must-not-execute']
            else: frame['records'][2]['output_bytes'] = 12
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                reader.traces(encode(frame), {'fixture'})

    def test_missing_duplicate_unexpected_and_oversized_streams_refuse(self):
        raw = encode(fixture())
        for data, allowed in [(b'', {'fixture'}), (raw+raw, {'fixture'}), (raw, {'other'}),
                               (b'x'*(16*1024**2+1), {'fixture'})]:
            with self.assertRaises(ValueError):
                reader.traces(data, allowed)


if __name__ == '__main__':
    unittest.main()
