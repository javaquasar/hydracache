import copy
import unittest
import review as r


def stats(**kwargs):
    value={k:0 for k in r.FIELDS};value.update(kwargs);return value


def value(variant):
    row=dict(threshold=64,repetitions=30,wire=[1,2,3],variant=variant,context_present=[False,False,False],
      CPU_qualified=False,RSS_qualified=False,allocator_page_retention_qualified=False)
    for point in r.POINTS:
        snap=dict(c=stats(allocs=1,requested=64,peak=64),rust=stats(allocs=3,requested=256,peak=256),errors=0)
        row[point]=[snap,dict(len=[0,0,0],capacity=[64,128,64])] if point in r.POINTS[:3] else snap
    return row


class Resource(unittest.TestCase):
    def test_identical_cost_is_only_a_subgate(self):
        result=r.reconcile(value('B0'),value('C2'))
        self.assertTrue(result['prerequisite_passed'])
        self.assertFalse(result['root_adopted'] or result['release_admitted'] or result['mtu_repaired'] or result['full_resource_contract_passed'])

    def test_lower_requests_cannot_buy_a_checkpoint_increase(self):
        before=value('B0');after=value('C2')
        for point in r.POINTS:
            snap=after[point][0] if point in r.POINTS[:3] else after[point]
            snap['c'].update(allocs=0,requested=0,peak=0)
        for point in r.POINTS[1:]:
            snap=after[point][0] if point in r.POINTS[:3] else after[point]
            snap['c'].update(allocs=1,requested=32,peak=32)
            if point in ('append1','append2'):snap['c']['live']=32
        result=r.reconcile(before,after)
        self.assertFalse(result['prerequisite_passed'])
        self.assertEqual([(v['point'],v['field']) for v in result['violations']],[('append1','live'),('append2','live')])

    def test_every_original_owner_field_and_capacity_is_a_veto(self):
        for owner in ('c','rust'):
            for field in ('allocs','realloc_calls','requested','peak'):
                before=value('B0');after=value('C2')
                for point in r.POINTS:
                    snap=after[point][0] if point in r.POINTS[:3] else after[point];snap[owner][field]+=1
                self.assertFalse(r.reconcile(before,after)['prerequisite_passed'])
        for i in range(3):
            before=value('B0');after=value('C2');after['append2'][1]['capacity'][i]+=1
            self.assertFalse(r.reconcile(before,after)['prerequisite_passed'])

    def test_missing_fields_wrong_source_scope_and_unbalanced_drop_refuse(self):
        changes=[lambda v:v['append1'][0]['c'].pop('live'),lambda v:v.update(variant='B0'),
          lambda v:v.update(context_present=[0,0,0]),lambda v:v.update(CPU_qualified=True),
          lambda v:v.update(wire=[3,2,1]),lambda v:v['retirement']['c'].update(live=1),
          lambda v:v['append1'][0]['rust'].update(requested=True),lambda v:v['append2'][0].update(errors=1)]
        for change in changes:
            before=value('B0');after=value('C2');change(after)
            with self.assertRaises((ValueError,KeyError)):r.reconcile(before,after)


if __name__=='__main__':unittest.main()
