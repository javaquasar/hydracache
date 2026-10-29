import copy
import json
import unittest
import capture


def fixture():
    rows = []
    for block in range(6):
        for path in capture.PATHS:
            for operation in capture.OPERATIONS if block % 2 == 0 else capture.OPERATIONS[::-1]:
                rows.append(dict(block=block,path=path,operation=operation,iterations=1,
                    wall_ns=100,parent_cpu_ns=0,actual_bytes=100,expected_bytes=100))
    prep = [dict(path=path,wall_ns=10,parent_cpu_ns=0,historical_cache_prepared=True,warmup=False) for path in capture.PATHS]
    observers = [dict(block=block,wall_ns=1,parent_cpu_ns=0) for block in range(6)]
    done = dict(blocks=6,samples=144,observer_samples=6,preparation_samples=3,historical_cache_prepared=True,
        warmup=False,component_intervals_are_disjoint=False,product=False,package_replay=False)
    return rows,prep,observers,done


def encode(parts):
    rows,prep,observers,done = parts
    return ('\n'.join(marker+json.dumps(value) for marker,values in
        [('HC074_NORMALIZATION_PREP ',prep),('HC074_NORMALIZATION ',rows),
         ('HC074_NORMALIZATION_OBSERVER ',observers),('HC074_NORMALIZATION_DONE ',[done])]
        for value in values)+'\n').encode()


class Controls(unittest.TestCase):
    def test_full_scope_not_additive_or_full_historical_chain(self):
        result = capture.review(encode(fixture()))
        self.assertEqual(len(result['samples']),144)
        self.assertEqual(len(result['summary']),24)
        self.assertFalse(result['component_intervals_are_disjoint'])
        self.assertFalse(result['full_historical_normalizer_chain'])

    def test_missing_duplicate_order_counts_times_types_and_source_drift_refuse(self):
        parts = fixture()
        mutations = []
        for mode in range(10):
            changed = copy.deepcopy(parts)
            if mode == 0: changed[0].pop()
            elif mode == 1: changed[0].append(changed[0][0])
            elif mode == 2: changed[0][0],changed[0][1] = changed[0][1],changed[0][0]
            elif mode == 3: changed[0][1]['wall_ns'] = -1
            elif mode == 4: changed[0][1]['iterations'] = True
            elif mode == 5: changed[0][1]['block'] = False
            elif mode == 6: changed[0][1]['actual_bytes'] += 1
            elif mode == 7: changed[1].pop()
            elif mode == 8: changed[2][0]['block'] = 1
            else: changed[3]['package_replay'] = 0
            mutations.append(changed)
        for changed in mutations:
            with self.assertRaises(ValueError): capture.review(encode(changed))


if __name__ == '__main__': unittest.main()
