"""Offline conservation and source-derived inventory, never a live guard."""
from pathlib import Path
import hashlib,importlib.util,json,re

ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('rank_profile',ROOT/'tools/native-server-control-074/profile_review.py')
profile=importlib.util.module_from_spec(spec);spec.loader.exec_module(profile)
require=profile.require
STEPS=('StorageBefore','ReservedLive','Progress','Causal','Prepared','Consent','StorageAfter','FinalClocks')
SOURCES=('diagnostic_worker_context.rs','diagnostic_worker_kernel_binding.rs','diagnostic_worker_trust_input.rs',
 'diagnostic_kernel_admission.rs','diagnostic_kernel_walk_trace.rs','diagnostic_live_walk_trace.rs','diagnostic_validation_frame.rs')

def sha(raw):return hashlib.sha256(raw).hexdigest()
def uint(value):return type(value) is int and value>=0
def strict_json(raw):
 def pairs(items):
  result={}
  for name,value in items:
   require(name not in result,'Duplicate JSON key');result[name]=value
  return result
 return json.loads(raw,object_pairs_hook=pairs,parse_constant=lambda _:require(False,'Nonfinite JSON'))
def load():
 protocol=strict_json((HERE/'protocol.json').read_bytes())
 base=ROOT/'docs/testing/performance/0.74/local-runs/authority-parent-cost-20261025'
 raw=(base/'index.json').read_bytes();require(sha(raw)==protocol['upstream_index_sha256'],'Changed parent index')
 index=strict_json(raw);require(len(index['members'])==291,'Changed parent inventory')
 def member(name):
  item=index['members'][name];digest=item['sha256'];require(re.fullmatch('[0-9a-f]{64}',digest),'Invalid object name')
  blob=(base/'objects'/digest).read_bytes();require(len(blob)==item['bytes'] and sha(blob)==digest,'Changed member bytes')
  return blob
 group='server/'+protocol['profile_group']+'/'
 files={n:member(group+n) for n in ['profile.stdout.log','profile.stderr.log','consumed.json','result.json','command.json','preflight.json','postflight.json']}
 files['source-manifest.json']=member('server/authority-parent-runtime-build-first/source-manifest.json')
 require(sha(files['source-manifest.json'])==protocol['source_manifest_sha256'],'Changed manifest')
 require(sha(files['profile.stdout.log'])==protocol['profile_stdout_sha256'],'Changed first profile')
 require(member('dependencies/profile-review.py')==(ROOT/'tools/native-server-control-074/profile_review.py').read_bytes(),'Changed base reader')
 manifest=strict_json(files['source-manifest.json']);require(len(manifest)==2565,'Changed source count')
 sources={name:(ROOT/'tools/long-run-supervisor-074/src'/name).read_bytes() for name in SOURCES}
 sources['round_cost.rs']=(ROOT/'tools/long-run-supervisor-074/tests/support/round_cost.rs').read_bytes()
 for name,blob in sources.items():
  relative=('tools/long-run-supervisor-074/tests/support/' if name=='round_cost.rs' else 'tools/long-run-supervisor-074/src/')+name
  require(sha(blob)==manifest[relative],'Current source differs from measured source: '+name)
 return files,sources,protocol

def span(value):
 require(set(value)=={'calls','inclusive_ns','outside_children_ns'},'Changed span shape')
 require(all(uint(v) for v in value.values()) and value['outside_children_ns']<=value['inclusive_ns'],'Invalid span')
 require(value['calls']>0 or value['inclusive_ns']==0,'Time without calls')

def balance(step):
 nodes=step['kernel'];edges=step['edges'];roots=step['roots']
 require(len(nodes)==len(roots)==10 and all(uint(v) for v in roots),'Changed kernel/root shape')
 for node in nodes:span(node)
 incoming=[0]*10;in_time=[0]*10;out_time=[0]*10;seen=set()
 for edge in edges:
  require(set(edge)=={'parent','child','calls','inclusive_ns'},'Changed edge shape')
  require(all(uint(v) for v in edge.values()),'Invalid edge types')
  p,c=edge['parent'],edge['child'];require(p<10 and c<10 and edge['calls']>0 and (p,c) not in seen,'Invalid/repeated edge')
  seen.add((p,c));incoming[c]+=edge['calls'];in_time[c]+=edge['inclusive_ns'];out_time[p]+=edge['inclusive_ns']
 root_time=[]
 for i,node in enumerate(nodes):
  require(node['calls']==incoming[i]+roots[i],'Unbalanced incoming calls')
  require(node['inclusive_ns']-node['outside_children_ns']==out_time[i],'Unbalanced direct-child time')
  require(in_time[i]<=node['inclusive_ns'],'Incoming time exceeds node')
  value=node['inclusive_ns']-in_time[i]
  require(roots[i]>0 or value==0,'Unrooted wall time');root_time.append(value)
 coverage=sum(root_time)
 require(coverage==sum(n['outside_children_ns'] for n in nodes) and coverage<=step['wall_ns'],'Unbalanced kernel coverage')
 return coverage

def expand(graph,node='observe',path=()):
 require(node not in path,'Recursive inventory')
 if node not in graph:return {node:1}
 require(type(graph[node]) is dict and graph[node],'Invalid inventory node')
 result={}
 for child,count in graph[node].items():
  require(type(count) is int and count>0,'Invalid inventory multiplier')
  for leaf,total in expand(graph,child,(*path,node)).items():result[leaf]=result.get(leaf,0)+count*total
 return dict(sorted(result.items()))

def analyze(raw,stderr):
 # Reject duplicate keys before invoking the older permissive base parser.
 require(len(raw)<8*1024**2 and len(stderr)<8*1024**2,'Oversized profile')
 for text in (raw+stderr).decode('utf-8').splitlines():
  for marker in ['HC074-HELD-PHASE ','HC074-HELD-LIVE-ORIGINAL ']:
   if marker in text:strict_json(text.split(marker,1)[1])
 base=profile.review(raw+stderr,101,101,0)
 require(base['first_failed_phase']=='enrollment' and len(base['phases'])==7 and not base['diagnostic_complete'],'Changed first frontier')
 rows=profile.records(raw,'HC074-HELD-PHASE ');phases=[]
 for row in rows:
  require(row['product'] is False,'Changed product scope')
  for name in ['authentication_calls','authentication_ns','signature_calls','signature_ns']:
   require(uint(row[name]),'Invalid authentication type')
  require(row['signature_ns']<=row['authentication_ns'],'Signature exceeds enclosing auth')
  require(len(row['round_steps'])==8,'Changed round shape');steps=[]
  for index,step in enumerate(row['round_steps']):
   require(type(step['step']) is int and step['step']==index,'Changed step order')
   require(all(uint(step[k]) for k in ['calls','completed','wall_ns']) and step['completed']<=step['calls'],'Invalid step counts')
   require(step['overlapping_spans'] is True and step['guard_clock_added'] is False,'Changed observer scope')
   coverage=balance(step)
   require(len(step['live'])==6,'Changed live shape')
   for value in step['live']:span(value)
   require(sum(v['outside_children_ns'] for v in step['live'])<=step['wall_ns'],'Live coverage exceeds slot')
   steps.append(dict(step=STEPS[index],calls=step['calls'],completed=step['completed'],wall_ns=step['wall_ns'],
    kernel_root_wall_ns=coverage,kernel_outside_root_wall_ns=step['wall_ns']-coverage))
  total=sum(s['wall_ns'] for s in steps);require(total<=row['wall_ns'],'Overlapping round slots')
  # Only bounded per-kind containment within this phase, never cross-view addition.
  for key,count in [('kernel',10),('live',6)]:
   require(len(row[key])==count,'Changed phase view shape')
   for i,value in enumerate(row[key]):
    span(value)
    for field in ['calls','inclusive_ns','outside_children_ns']:
     require(sum(s[key][i][field] for s in row['round_steps'])<=value[field],'Round counters exceed phase')
   require(sum(v['outside_children_ns'] for v in row[key])<=row['wall_ns'],'View exceeds phase')
  outer=row['live'][4];inner=row['kernel'][0]
  require(outer['calls']==inner['calls'] and outer['inclusive_ns']>=inner['inclusive_ns'],'KernelContext/PacketContext wrapper mismatch')
  views={key:sorted([dict(kind=name,**value) for name,value in zip(names,row[key])],key=lambda r:r['outside_children_ns'],reverse=True)
   for key,names in [('kernel',profile.KERNEL),('live',profile.LIVE)]}
  phases.append(dict(phase=row['phase'],returned=row['returned'],wall_ns=row['wall_ns'],process_cpu_ns=row['process_cpu_ns'],
   round_slots=steps,round_wall_ns=total,phase_wall_outside_rounds_ns=row['wall_ns']-total,ranked_views=views,
   kernel_context_wraps_packet_context=True,kernel_context_outer_minus_inner_wall_ns=outer['inclusive_ns']-inner['inclusive_ns']))
 return dict(phases=phases,first_original_refusal=base['first_original_refusal'],first_failed_phase=base['first_failed_phase'],
  not_observed_phases=base['not_observed_phases'],all_observed_round_trees_balanced=True,
  no_sum_across_views=True,wall_is_not_cpu_or_io=True,observer_cost_not_subtracted=True,
  measurements_repeated=False,optimization_implemented=False,performance_improvement_proven=False,
  original_windows_seconds=[300,60,10],server=False,product=False,admission=False)

def review(files,sources,protocol,graph):
 require(sha(files['source-manifest.json'])==protocol['source_manifest_sha256'],'Changed measured manifest')
 require(sha(files['profile.stdout.log'])==protocol['profile_stdout_sha256'],'Changed first profile bytes')
 result=strict_json(files['result.json']);intent=strict_json(files['consumed.json']);command=strict_json(files['command.json'])
 require(result['exit']==101 and result['first_refusal'] is None and result['postflight_refusal'] is None,'Changed profile result')
 require(result['attribution_only'] and not result['functional_gate'] and not result['admission'],'Promoted profile')
 require(intent['source_head']==protocol['source_commit'] and intent['image_sha256']==protocol['image_sha256'],'Changed original pins')
 require(intent['original_windows']==[300,60,10] and not intent['retry'] and intent['attribution_only'] and not intent['healthy_gate'],'Changed original scope')
 require(command['env']['HC074_CONSUMING_HELD_COST']=='1' and command['env']['HC074_CONSUMING_HELD_LIVE_CASE']=='healthy'
  and 'HC074_HELD_MATERIAL_LIVE' not in command['env'],'Changed profile dispatch')
 require(strict_json(files['preflight.json'])==strict_json(files['postflight.json']),'Changed host frame')
 manifest=strict_json(files['source-manifest.json'])
 for name,blob in sources.items():
  relative=('tools/long-run-supervisor-074/tests/support/' if name=='round_cost.rs' else 'tools/long-run-supervisor-074/src/')+name
  require(sha(blob)==manifest[relative],'Source drift: '+name)
 inventory=expand(graph);require(inventory==protocol['successful_local_context_expected_requests'],'Changed source-derived inventory')
 facts=analyze(files['profile.stdout.log'],files['profile.stderr.log'])
 output=next(p for p in facts['phases'] if p['phase']=='outputs')
 require(all(s['calls']==s['completed']==46 for s in output['round_slots']),'Changed first output schedule')
 live={s['kind']:s for s in output['ranked_views']['live']};calls=live['LocalContext']['calls']
 facts['local_context_inventory']=dict(per_successful_observe=inventory,observed_span_calls=calls,
  source_derived_output_requests={k:v*calls for k,v in inventory.items()},directly_measured_request_counts=False,
  candidate_scope='only the fresh directory metadata projection in PinnedDirectory::same; no constructor/document/namespace changes',
  candidate_implemented=False,cpu_saving_predicted=False)
 return facts

if __name__=='__main__':
 files,sources,protocol=load();graph=strict_json((HERE/'local-context-graph.json').read_bytes())
 print(json.dumps(review(files,sources,protocol,graph),sort_keys=True,indent=2))
