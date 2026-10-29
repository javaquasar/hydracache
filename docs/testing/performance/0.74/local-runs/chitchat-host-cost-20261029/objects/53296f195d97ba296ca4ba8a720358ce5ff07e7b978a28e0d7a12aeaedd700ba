"""Inert fault controls for case accounting; no native/performance command."""
import copy
import json
import unittest

import analyze
import common as c
import run


class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        rows=run.diagnostic(c.member('capture/delta-B-194/stdout.log').decode(),'HC074_SUPPORTED_DELTA:')['cases']
        cls.rows=[dict(v,records=[dict(complete_counts=dict(allocs=1,reallocs=0,requested=8,live=0,peak=8),actual_calls=[2,0,0,0])]*2) for v in rows]

    def cohort(self):
        value=dict(seed=0x0741028,native_C_allocations_included=False,rows=self.rows,actual_call_order=analyze.CALL_ORDER)
        return [copy.deepcopy(value) for _ in range(4)]

    def candidate(self,values,mutate):
        for i in (1,2):mutate(values[i])

    def test_complete_equal_cohort_is_bounded_rust_result(self):
        result=analyze.reconcile(self.cohort(),'P')
        self.assertTrue(result['rust_allocation_gate_pass'])
        for key in ('CPU_qualified','native_C_allocations_qualified','RSS_qualified','retained_memory_qualified','root_adopted'):
            self.assertFalse(result[key])

    def test_admission_and_wire_change_cannot_pass(self):
        for key,value in (('accepted',False),('wire','00')):
            values=self.cohort()
            self.candidate(values,lambda v:v['rows'][0].update({key:value}))
            with self.assertRaises(ValueError):analyze.reconcile(values,'P')

    def test_order_and_missing_case_cannot_pass(self):
        for mutate in (lambda v:v['rows'].reverse(),lambda v:v['rows'].pop()):
            values=self.cohort();self.candidate(values,mutate)
            with self.assertRaises(ValueError):analyze.reconcile(values,'P')

    def test_cross_process_and_inner_repeats_must_be_exact(self):
        values=self.cohort();values[1]['rows'][0]['records'][0]=copy.deepcopy(values[1]['rows'][0]['records'][0])
        values[1]['rows'][0]['records'][0]['actual_calls'][0]=3
        with self.assertRaises(ValueError):analyze.reconcile(values,'P')
        values=self.cohort();values[1]['rows'][0]['records'][0]['actual_calls'][0]=3
        with self.assertRaises(ValueError):analyze.reconcile(values,'P')

    def test_each_allocation_field_can_independently_refuse(self):
        for key in analyze.FIELDS:
            values=self.cohort()
            def mutate(v):
                v['rows'][0]['records'][0]['complete_counts'][key]+=1
            self.candidate(values,mutate)
            result=analyze.reconcile(values,'P')
            self.assertFalse(result['rust_allocation_gate_pass']);self.assertEqual(result['regressions'],1)

    def test_live_and_invalid_counter_values_refuse(self):
        for key,value in (('live',1),('live',False),('peak',-1),('allocs',True),('requested',0.5)):
            values=self.cohort()
            for v in values:v['rows'][0]['records'][0]['complete_counts'][key]=value
            with self.assertRaises(ValueError):analyze.reconcile(values,'P')

    def test_seed_and_allocator_scope_cannot_drift(self):
        for key,value in (('seed',1),('native_C_allocations_included',True)):
            values=self.cohort();self.candidate(values,lambda v:v.update({key:value}))
            with self.assertRaises(ValueError):analyze.reconcile(values,'P')

    def test_filtered_ignored_failed_or_partial_test_results_refuse(self):
        self.assertEqual(run.accounting('test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 116 filtered out;',3,119)['passed'],3)
        for raw in ('test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 116 filtered out;',
                    'test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 116 filtered out;',
                    'test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 115 filtered out;',
                    'running 3 tests'):
            with self.assertRaises(ValueError):run.accounting(raw,3,119)

    def test_diagnostic_is_unique_and_complete(self):
        self.assertEqual(run.diagnostic('M:{"complete":true}','M:'),{'complete':True})
        for raw in ('','M:{}\nM:{}','M:{'):
            with self.assertRaises((ValueError,json.JSONDecodeError)):run.diagnostic(raw,'M:')

    def test_original_semantic_oracle_rejects_different_payload(self):
        run.semantics(self.rows)
        rows=copy.deepcopy(self.rows);rows[0]['value_len']+=1
        with self.assertRaises(ValueError):run.semantics(rows)

    def test_path_counter_header_cannot_be_reinterpreted(self):
        values=self.cohort()
        self.candidate(values,lambda v:v['actual_call_order'].reverse())
        with self.assertRaises(ValueError):analyze.reconcile(values,'P')

    def test_portable_observer_only_changes_platform_import(self):
        old=(c.TOOLS/'allocation_tests.rs').read_bytes()
        self.assertEqual(old.count(b'use std::time::Instant;'),1)
        self.assertEqual((c.TOOLS/'allocation_tests_portable.rs').read_bytes(),old.replace(b'use std::time::Instant;',b'#[cfg(windows)]\nuse std::time::Instant;'))

    def test_final_production_matches_measured_P(self):
        old=c.BASE/'fixtures/P';new=c.BASE/'fixtures/L3'
        before=(old/'src/serialize.rs').read_text(encoding='utf8')
        after=(new/'src/serialize.rs').read_text(encoding='utf8')
        for marker in ('    fn counted_upperbound_after<','    pub(crate) fn fits_budget_after<',
                       '    pub fn serialized_len_upperbound_after<','    pub fn append<',
                       '    fn flush_block(', '    pub fn finish(mut self)'):
            a,b=c.function_span(before,marker);x,y=c.function_span(after,marker)
            self.assertEqual(before[a:b],after[x:y],marker)
        self.assertEqual((old/'src/delta.rs').read_bytes(),(new/'src/delta.rs').read_bytes().replace(b'allocation_tests_portable.rs',b'allocation_tests.rs'))
        for name in ('Cargo.toml','Cargo.lock'):
            self.assertEqual((old/name).read_bytes(),(new/name).read_bytes())


if __name__=='__main__':unittest.main()
