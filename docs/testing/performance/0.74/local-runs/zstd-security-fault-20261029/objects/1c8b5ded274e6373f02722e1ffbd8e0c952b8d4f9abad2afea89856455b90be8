import copy,json,random,unittest
import review as api

FILES,SOURCES,PROTOCOL=api.load()
GRAPH=api.strict_json((api.HERE/'local-context-graph.json').read_bytes())
RAW=FILES['profile.stdout.log'];STDERR=FILES['profile.stderr.log']
def rows():return api.profile.records(RAW,'HC074-HELD-PHASE ')
def changed_rows(values):
 original=api.profile.records(RAW,'HC074-HELD-LIVE-ORIGINAL ')
 return ('\n'.join('HC074-HELD-PHASE '+json.dumps(v) for v in values)+'\nHC074-HELD-LIVE-ORIGINAL '+json.dumps(original[0])+'\n').encode()

class Controls(unittest.TestCase):
 def test_exact_first_profile_preserves_red_frontier_and_separate_views(self):
  value=api.review(FILES,SOURCES,PROTOCOL,GRAPH)
  self.assertEqual(value['first_failed_phase'],'enrollment')
  self.assertEqual(value['first_original_refusal']['age_ns'],10005376028)
  self.assertTrue(value['all_observed_round_trees_balanced'])
  self.assertEqual(value['not_observed_phases'],['placement','fixed-slice','consuming'])
  self.assertTrue(value['no_sum_across_views'])
  self.assertFalse(value['performance_improvement_proven']);self.assertFalse(value['admission'])
  self.assertEqual(value['local_context_inventory']['source_derived_output_requests']['directory_same'],101568)
 def test_typed_counts_and_booleans_refuse(self):
  for field,value in [('step',True),('calls',True),('completed',47),('wall_ns',-1)]:
   changed=rows();changed[2]['round_steps'][0][field]=value
   with self.subTest(field=field),self.assertRaises(ValueError):api.analyze(changed_rows(changed),STDERR)
 def test_missing_and_reordered_slots_refuse(self):
  changed=rows();changed[2]['round_steps'].pop()
  with self.assertRaises(ValueError):api.analyze(changed_rows(changed),STDERR)
  changed=rows();changed[2]['round_steps'].reverse()
  with self.assertRaises(ValueError):api.analyze(changed_rows(changed),STDERR)
 def test_incoming_roots_and_child_time_conservation_refuse(self):
  for mutation in ['edge-call','edge-time','root','node-time','node-call','negative','bool']:
   changed=copy.deepcopy(rows()[2]['round_steps'][1])
   if mutation=='edge-call':changed['edges'][0]['calls']+=1
   if mutation=='edge-time':changed['edges'][0]['inclusive_ns']+=1
   if mutation=='root':changed['roots'][0]+=1
   if mutation=='node-time':changed['kernel'][0]['inclusive_ns']+=1
   if mutation=='node-call':changed['kernel'][0]['calls']+=1
   if mutation=='negative':changed['kernel'][0]['outside_children_ns']=-1
   if mutation=='bool':changed['edges'][0]['parent']=False
   with self.subTest(mutation=mutation),self.assertRaises(ValueError):api.balance(changed)
 def test_duplicate_out_of_bounds_or_zero_edges_refuse(self):
  for mutation in ['duplicate','bounds','zero']:
   changed=copy.deepcopy(rows()[2]['round_steps'][1])
   if mutation=='duplicate':changed['edges'].append(dict(changed['edges'][0]))
   if mutation=='bounds':changed['edges'][0]['child']=10
   if mutation=='zero':changed['edges'][0]['calls']=0
   with self.subTest(mutation=mutation),self.assertRaises(ValueError):api.balance(changed)
 def test_balanced_tree_still_cannot_exceed_wall_budget(self):
  step=copy.deepcopy(rows()[2]['round_steps'][1]);coverage=api.balance(step);step['wall_ns']=coverage-1
  with self.assertRaises(ValueError):api.balance(step)
 def test_phase_slot_counter_and_wrapper_containment_refuse(self):
  for mutation in ['phase-time','counter','wrapper']:
   changed=rows()
   if mutation=='phase-time':changed[2]['wall_ns']=1
   if mutation=='counter':changed[2]['kernel'][0]['calls']=0
   if mutation=='wrapper':changed[2]['live'][4]['calls']+=1
   with self.subTest(mutation=mutation),self.assertRaises(ValueError):api.analyze(changed_rows(changed),STDERR)
 def test_json_duplicates_and_nonfinite_refuse(self):
  for raw in ['{"calls":1,"calls":1}','{"time":NaN}','{"time":Infinity}']:
   with self.assertRaises(ValueError):api.strict_json(raw)
 def test_dag_multiplication_shared_nodes_and_cycles(self):
  self.assertEqual(api.expand(GRAPH),PROTOCOL['successful_local_context_expected_requests'])
  self.assertEqual(api.expand({'observe':{'a':2,'b':3},'a':{'metadata':5},'b':{'a':4}}),{'metadata':70})
  for graph in [{'observe':{'observe':1}},{'observe':{'a':0}},{'observe':{'a':True}}]:
   with self.assertRaises(ValueError):api.expand(graph)
 def test_seeded_acyclic_inventory_matches_literal_expansion(self):
  rng=random.Random(0x074ca117)
  for _ in range(128):
   graph={'observe':{'a':rng.randint(1,8),'b':rng.randint(1,8)},'a':{'metadata':rng.randint(1,8)},'b':{'a':rng.randint(1,8),'read_at':rng.randint(1,8)}}
   expected={'metadata':graph['observe']['a']*graph['a']['metadata']+graph['observe']['b']*graph['b']['a']*graph['a']['metadata'],
    'read_at':graph['observe']['b']*graph['b']['read_at']}
   self.assertEqual(api.expand(graph),expected)
 def test_changed_source_pins_manifest_dispatch_host_or_scope_refuse(self):
  changed=dict(SOURCES);changed['diagnostic_worker_context.rs']+=b'\n'
  with self.assertRaises(ValueError):api.review(FILES,changed,PROTOCOL,GRAPH)
  for name,field,value in [('consumed.json','source_head','changed'),('consumed.json','retry',True),
   ('consumed.json','original_windows',[300,60,11]),('result.json','functional_gate',True)]:
   changed=dict(FILES);body=api.strict_json(changed[name]);body[field]=value;changed[name]=json.dumps(body).encode()
   with self.subTest(name=name,field=field),self.assertRaises(ValueError):api.review(changed,SOURCES,PROTOCOL,GRAPH)
  changed=dict(FILES);changed['source-manifest.json']+=b'\n'
  with self.assertRaises(ValueError):api.review(changed,SOURCES,PROTOCOL,GRAPH)
  changed=dict(FILES);changed['postflight.json']=b'{}'
  with self.assertRaises(ValueError):api.review(changed,SOURCES,PROTOCOL,GRAPH)
  changed=dict(FILES);body=api.strict_json(changed['command.json']);body['env']['HC074_HELD_MATERIAL_LIVE']='1';changed['command.json']=json.dumps(body).encode()
  with self.assertRaises(ValueError):api.review(changed,SOURCES,PROTOCOL,GRAPH)
 def test_changed_inventory_cannot_supply_the_next_baseline(self):
  graph=copy.deepcopy(GRAPH);graph['named']['same-directory']=5
  with self.assertRaises(ValueError):api.review(FILES,SOURCES,PROTOCOL,graph)

if __name__=='__main__':unittest.main()
