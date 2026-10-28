import copy
import unittest

import compare as c


def case(parsed=True):
    value = dict(id='one', parsed=True, tree={'ordered': [1, 2]}, roundtrip={'ok': True}) if parsed else \
        dict(id='one', parsed=False, message='refused', location={'line': 1, 'column': 2, 'index': 1})
    return dict(id='one', exit=0, timeout=False, value=value)


class Comparison(unittest.TestCase):
    def test_equal_success_and_refusal(self):
        for parsed in (True, False):
            row = case(parsed)
            result = c.compare([dict(id='one', expect='valid' if parsed else 'invalid')], [row], [copy.deepcopy(row)])
            self.assertTrue(result['semantic_contract_pass'])
            self.assertFalse(result['diagnostic_review_required'] or result['release_admitted'])

    def test_decision_and_declared_expectation_are_not_relaxed(self):
        result = c.compare([dict(id='one', expect='valid')], [case()], [case(False)])
        self.assertFalse(result['semantic_contract_pass'])
        result = c.compare([dict(id='one', expect='invalid')], [case()], [case()])
        self.assertFalse(result['semantic_contract_pass'])

    def test_values_order_and_serialization_are_exact(self):
        new = case()
        new['value']['tree']['ordered'].reverse()
        self.assertFalse(c.compare([dict(id='one', expect='valid')], [case()], [new])['semantic_contract_pass'])

    def test_diagnostic_drift_is_retained_and_requires_review(self):
        new = case(False)
        new['value']['message'] = 'new diagnostic'
        result = c.compare([dict(id='one', expect='invalid')], [case(False)], [new])
        self.assertTrue(result['semantic_contract_pass'] and result['diagnostic_review_required'])
        self.assertEqual(result['diagnostic_differences'][0]['candidate'], new['value'])

    def test_missing_unknown_duplicate_and_reordered_cases_refuse(self):
        for ids in ([], [case(), case()], [dict(case(), id='other')]):
            with self.assertRaises(ValueError):
                c.compare([dict(id='one', expect='valid')], [case()], ids)
        two = case()
        two['id'] = two['value']['id'] = 'two'
        with self.assertRaises(ValueError):
            c.compare([dict(id='one', expect='valid'), dict(id='two', expect='valid')],
                      [case(), two], [two, case()])

    def test_process_crash_and_timeout_refuse(self):
        for field, value in (('exit', 1), ('exit', False), ('timeout', True)):
            bad = case()
            bad[field] = value
            with self.assertRaises(ValueError):
                c.compare([dict(id='one', expect='valid')], [case()], [bad])

    def test_unknown_response_fields_and_nonboolean_decision_refuse(self):
        for changed in (dict(case()['value'], unreviewed=True), dict(case()['value'], parsed=1)):
            bad = case()
            bad['value'] = changed
            with self.assertRaises(ValueError):
                c.compare([dict(id='one', expect='valid')], [case()], [bad])
