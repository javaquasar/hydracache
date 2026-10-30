import copy
import unittest
import analyze as a


def stats(**values):
    row={k:0 for k in a.FIELDS};row.update(values);return row


def fixture():
    v=dict(threshold=64,repetitions=30,wire=[1,2,3],variant='B0',context_present=[False,False,False],CPU_qualified=False,RSS_qualified=False,allocator_page_retention_qualified=False)
    for point in a.CHECKPOINTS:
        snap=dict(c=stats(allocs=1,requested=64,peak=64),rust=stats(allocs=3,requested=256,peak=256),errors=0)
        v[point]=[snap,dict(len=[0,0,0],capacity=[64,128,64])] if point in a.CHECKPOINTS[:3] else snap
    return v


class Admission(unittest.TestCase):
    def test_identical_input_is_only_a_prerequisite(self):
        row=a.reconcile(fixture(),fixture())
        self.assertTrue(row['prerequisite_passed'])
        self.assertFalse(row['root_adopted'] or row['release_admitted'] or row['mtu_repaired'])

    def test_lower_total_c_requests_do_not_buy_live_retention(self):
        before=fixture();after=copy.deepcopy(before)
        for point in a.CHECKPOINTS:
            snap=after[point][0] if point in a.CHECKPOINTS[:3] else after[point]
            snap['c'].update(allocs=0,requested=0,peak=0)
        for point in ('append1','append2','finish','retirement'):
            snap=after[point][0] if point in a.CHECKPOINTS[:3] else after[point]
            snap['c'].update(allocs=1,requested=32,peak=32)
            if point in ('append1','append2'):snap['c']['live']=32
        row=a.reconcile(before,after)
        self.assertFalse(row['prerequisite_passed'])
        self.assertEqual([(v['point'],v['field']) for v in row['violations']],[('append1','live'),('append2','live')])
        self.assertEqual(row['decision'],'declined-context-retention-stop-combined-candidate')

    def test_every_checkpoint_field_is_a_veto(self):
        for point in a.CHECKPOINTS:
            for owner in ('c','rust'):
                for field in ('allocs','realloc_calls','requested','peak'):
                    b=fixture();c=copy.deepcopy(b)
                    # Preserve monotonicity in this independent corrupt input.
                    for later in a.CHECKPOINTS[a.CHECKPOINTS.index(point):]:
                        snap=c[later][0] if later in a.CHECKPOINTS[:3] else c[later]
                        snap[owner][field]+=1
                    self.assertFalse(a.reconcile(b,c)['prerequisite_passed'])

    def test_missing_counter_changed_work_scope_and_unbalanced_drop_refuse(self):
        mutations=[lambda v:v['append1'][0]['c'].pop('live'),
          lambda v:v['retirement']['c'].update(live=1),
          lambda v:v['append1'][0]['c'].update(errors=1),
          lambda v:v.update(RSS_qualified=True),lambda v:v.update(wire=[3,2,1]),
          lambda v:v['append2'][1]['len'].__setitem__(0,65),
          lambda v:v['append1'][0]['c'].update(requested=True)]
        for change in mutations:
            before=fixture();after=copy.deepcopy(before);change(after)
            with self.assertRaises((ValueError,KeyError)):a.reconcile(before,after)


if __name__=='__main__':unittest.main()
