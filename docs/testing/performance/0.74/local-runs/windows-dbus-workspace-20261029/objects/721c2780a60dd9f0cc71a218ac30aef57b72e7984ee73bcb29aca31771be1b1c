import copy
import json
import unittest

import compare as c


def snapshot(isolated):
    ids = [f'{t}/{n}/{k}' for t in (16, 200, 1000, 65535) for n in (0, 1, 100) for k in range(3)]+['max-string']
    context = dict(output=b'second job'.hex() if isolated else b'first jobsecond job'.hex(), isolated=isolated)
    return dict(seed=0x0741028, cases=[dict(id=i, wire='00') for i in ids],
        faults=[dict(id=i, input='ff', accepted=False, message='refused', remaining=0) for i in range(7)],
        encoder=copy.deepcopy(context), decoder=copy.deepcopy(context), native_version='1.5.7')


class Comparison(unittest.TestCase):
    def test_zero_missing_duplicate_or_wrong_native_accounting_refuses(self):
        good = b'test context_review_074::snapshot ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 101 filtered out; finished in 0.00s\n'
        c.require_selected_test(good, 'context_review_074::snapshot', 102)
        for raw in (b'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 101 filtered out; finished in 0.00s\n',
                    good+good, good.replace(b'101 filtered', b'100 filtered'), good.replace(b'context_review', b'other_review')):
            with self.assertRaises(ValueError): c.require_selected_test(raw, 'context_review_074::snapshot', 102)

    def test_independent_context_control_never_admits_wire_or_release(self):
        old = {k:v for k,v in snapshot(False).items() if k in ('encoder', 'decoder')}
        new = {k:v for k,v in snapshot(True).items() if k in ('encoder', 'decoder')}
        result = c.compare_context(old, new)
        self.assertTrue(result['encoder_repaired'] and result['decoder_repaired'])
        self.assertFalse(result['full_wire_gate_pass'] or result['release_admitted'])
        self.assertEqual(c.parse_context(('HC074_ZSTD_CONTEXT:'+json.dumps(new)).encode()), new)
        with self.assertRaises(ValueError): c.compare_context(new, new)
        with self.assertRaises(ValueError): c.parse_context(b'')

    def test_exact_wire_and_paired_security_change(self):
        self.assertTrue(c.compare(snapshot(False), snapshot(True))['semantic_contract_pass'])

    def test_missing_duplicate_unknown_seed_and_wire_drift_refuse(self):
        for change in ('missing', 'duplicate', 'wire', 'seed', 'extra', 'native'):
            new = snapshot(True)
            if change == 'missing': new['cases'].pop()
            elif change == 'duplicate': new['cases'][1] = new['cases'][0]
            elif change == 'wire': new['cases'][0]['wire'] = '01'
            elif change == 'seed': new['seed'] = True
            elif change == 'extra': new['unreviewed'] = True
            else: new['native_version'] = 'different'
            with self.subTest(change=change), self.assertRaises(ValueError):
                c.compare(snapshot(False), new)

    def test_unknown_fault_or_accepted_corruption_refuses(self):
        for change in ('id', 'accepted', 'remaining', 'extra'):
            new = snapshot(True)
            if change == 'id': new['faults'][0]['id'] = 9
            elif change == 'accepted': new['faults'][0]['accepted'] = True
            elif change == 'remaining': new['faults'][0]['remaining'] = 1
            else: new['faults'][0]['unreviewed'] = True
            with self.assertRaises(ValueError): c.compare(snapshot(False), new)

    def test_diagnostic_drift_is_retained_for_review(self):
        new = snapshot(True); new['faults'][0]['message'] = 'changed wording'
        result = c.compare(snapshot(False), new)
        self.assertTrue(result['diagnostic_review_required'])
        self.assertEqual(result['diagnostic_differences'][0]['candidate'], new['faults'][0])

    def test_both_contexts_and_exact_second_job_are_mandatory(self):
        for name in ('encoder', 'decoder'):
            for field, value in (('isolated', False), ('isolated', 1), ('output', b'wrong'.hex())):
                new = snapshot(True); new[name][field] = value
                with self.assertRaises(ValueError): c.compare(snapshot(False), new)
        with self.assertRaises(ValueError): c.compare(snapshot(True), snapshot(True))

    def test_missing_or_multiple_native_snapshots_refuse(self):
        row = (c.MARKER+json.dumps(snapshot(False))+'\n').encode()
        self.assertEqual(c.parse(row), snapshot(False))
        for raw in (b'', row+row, b'\xff', (c.MARKER+'{} {}').encode()):
            with self.assertRaises((ValueError, UnicodeError)): c.parse(raw)
