"""Pure operation-attribution reader. Selected spans include clock overhead."""
from pathlib import Path
import importlib.util,math,statistics
spec=importlib.util.spec_from_file_location('operation_math',Path(__file__).parents[1]/'authority-directory-cost-074/review.py')
common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
require=common.require
CASES=('boot','user','mnt','pid','time')
STAGES=('open','filesystem','identity','document','decode','path','empty')
ARMS=('reference','reference-control','runtime','observed')
COUNTS=((4,4,0,1,1,0,1),(1,1,1,0,0,1,1))
def parse(raw,cohort):
 require(raw.decode('ascii').count('test result: ok. 1 passed; 0 failed; 0 ignored;')==1,'Missing verdict')
 origins=common.records(raw,'HC074-OPERATION-ORIGIN ')
 require(len(origins)==1 and origins[0]==dict(cohort=cohort,uid=1000,samples=16,iterations=256,warmups=16,product=False,admission=False),'Changed origin')
 origin=origins[0]
 require(all(type(origin[k]) is int for k in ('cohort','uid','samples','iterations','warmups')) and origin['product'] is False and origin['admission'] is False,'Changed origin types')
 rows=common.records(raw,'HC074-OPERATION-COST ')
 require(len(rows)==2240,'Incomplete/repeated work')
 for index,row in enumerate(rows):
  case,tail=divmod(index,448);stage,tail=divmod(tail,64);sample,slot=divmod(tail,4)
  require(row['case']==CASES[case] and row['stage']==STAGES[stage],'Changed case/stage order')
  require(all(type(row[k]) is int and row[k]==v for k,v in [('cohort',cohort),('sample',sample),('slot',slot),('calls',256)]),'Changed work/types')
  variant=ARMS[common.ORDERS[(sample+cohort)%8][slot]]
  require(row['variant']==variant,'Changed arm order')
  calls=COUNTS[int(case!=0)][stage]*256 if variant=='observed' else 0
  require(type(row['operation_calls']) is int and row['operation_calls']==calls,'Changed operation counts')
  require(type(row['operation_cpu_ns']) is int and (row['operation_cpu_ns']>0 if calls else row['operation_cpu_ns']==0),'Invalid operation CPU')
  require(row['clock_failed'] is False and row['product'] is False and row['admission'] is False,'Changed flags/clock failure')
  require(all(type(row[k]) is int and row[k]>0 for k in ('cpu_ns','wall_ns')),'Invalid whole clock')
 return rows
def review(cohorts):
 require(len(cohorts)==8,'Changed process count')
 parsed=[parse(raw,i) for i,raw in enumerate(cohorts)]
 cases={}
 for case in CASES:
  stages={}
  for stage in STAGES:
   estimates={arm:[] for arm in ARMS[1:]};selected=[];reference=[]
   for rows in parsed:
    rows=[r for r in rows if r['case']==case and r['stage']==stage]
    pairs={arm:[] for arm in ARMS[1:]};spans=[];times=[]
    for start in range(0,64,4):
     chunk=rows[start:start+4];cpu={r['variant']:r['cpu_ns'] for r in chunk}
     for arm in pairs:pairs[arm].append(math.log(cpu[arm]/cpu['reference']))
     observed=next(r for r in chunk if r['variant']=='observed')
     if observed['operation_calls']:spans.append(observed['operation_cpu_ns']/observed['operation_calls'])
     times.append(cpu['reference']/256)
    for arm in pairs:estimates[arm].append(statistics.median(pairs[arm]))
    if spans:selected.append(statistics.median(spans))
    reference.append(statistics.median(times))
   ratios={arm:common.interval(values) for arm,values in estimates.items()}
   stages[stage]=dict(ratios=ratios,controls_passed=common.inside(ratios['reference-control']) and common.inside(ratios['runtime']),
    selected_cpu_ns_per_operation=statistics.median(selected) if selected else None,
    selected_process_medians=selected,reference_cpu_ns_per_call=statistics.median(reference))
  cases[case]=stages
 return dict(cases=cases,calibration_passed=all(stage['controls_passed'] for stages in cases.values() for stage in stages.values()),
  selected_spans_include_observer_cost=True,empty_control_subtracted=False,spans_additive=False,
  performance_improvement_proven=False,preparation_retry=False,product=False,admission=False)
