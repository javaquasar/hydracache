import copy
import json
import unittest
import history_capture as capture


def fixture():
    prep = [dict(path=path, wall_ns=1, input_bytes=100, input_sha256='a' * 64,
                 prefix_warmup=False) for path in capture.PATHS]
    rows = [dict(block=block, path=path, operation=operation, iterations=16,
                 wall_ns=160, parent_cpu_ns=0, input_bytes=100, output_bytes=90,
                 output_sha256='b' * 64)
            for block in range(6) for path in capture.PATHS
            for operation in (capture.OPERATIONS if block % 2 == 0 else capture.OPERATIONS[::-1])]
    done = dict(samples=72, blocks=6, iterations=16, prefix_warmup=False,
                disjoint_intervals=False, full_fixed_slice_chain=False, product=False, package_replay=False)
    return prep, rows, done


def encode(parts):
    prep, rows, done = parts
    return ('\n'.join(marker + json.dumps(row)
        for marker, values in [('HC074_HISTORY_PREP ', prep), ('HC074_HISTORY ', rows),
                               ('HC074_HISTORY_DONE ', [done])] for row in values) + '\n').encode()


class Controls(unittest.TestCase):
    def test_namespace_scope_cannot_be_confused_with_prefix_cohort(self):
        prep, rows, done = fixture()
        rows = [dict(row, operation='namespace-chain') for row in rows if row['operation'] == capture.OPERATIONS[0]]
        done['samples'] = 18
        raw = encode((prep, rows, done))
        result = capture.review(raw, namespace=True)
        self.assertTrue(result['namespace_chain'])
        self.assertEqual(len(result['summary']), 3)
        with self.assertRaises(ValueError):
            capture.review(raw)
        with self.assertRaises(ValueError):
            capture.review(encode(fixture()), namespace=True)

    def test_exact_scope_and_no_subtraction(self):
        result = capture.review(encode(fixture()))
        self.assertEqual(len(result['samples']), 72)
        self.assertEqual(len(result['summary']), 12)
        self.assertEqual(next(iter(result['summary'].values()))['wall_ns']['median_per_call'], 10)
        self.assertFalse(result['full_fixed_slice_chain'])
        self.assertTrue(result['nested_prefixes_not_disjoint'])
        self.assertFalse(result['observer_subtracted'])

    def test_scope_order_types_digests_and_drift_refuse(self):
        for mode in range(13):
            changed = copy.deepcopy(fixture())
            if mode == 0: changed[1].pop()
            elif mode == 1: changed[1].append(changed[1][0])
            elif mode == 2: changed[1][0], changed[1][1] = changed[1][1], changed[1][0]
            elif mode == 3: changed[1][0]['block'] = False
            elif mode == 4: changed[1][0]['iterations'] = True
            elif mode == 5: changed[1][0]['wall_ns'] = -1
            elif mode == 6: changed[1][12]['output_sha256'] = 'c' * 64
            elif mode == 7: changed[1][12]['output_bytes'] = 91
            elif mode == 8: changed[1][0]['input_bytes'] = 101
            elif mode == 9: changed[0][0]['input_sha256'] = 'z' * 64
            elif mode == 10: changed[0].pop()
            elif mode == 11: changed[2]['product'] = 0
            else: changed[0][0]['prefix_warmup'] = True
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                capture.review(encode(changed))


if __name__ == '__main__':
    unittest.main()
