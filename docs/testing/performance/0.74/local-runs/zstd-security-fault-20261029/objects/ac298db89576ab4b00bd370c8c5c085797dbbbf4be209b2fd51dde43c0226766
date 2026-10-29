"""Pure seven-case filesystem buffer comparison, never an admission input."""
from pathlib import Path
import importlib.util,math,statistics
spec=importlib.util.spec_from_file_location('fs_math',Path(__file__).parents[1]/'authority-directory-cost-074/review.py')
common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
require=common.require
CASES=('proc-fs','ns-fs','boot','user','mnt','pid','time')
ARMS=common.ARMS
def parse(raw,cohort,phase):
 require(raw.decode('ascii').count('test result: ok. 1 passed; 0 failed; 0 ignored;')==1,'Missing verdict')
 origins=common.records(raw,'HC074-FILESYSTEM-ORIGIN ')
 require(len(origins)==1 and origins[0]==dict(cohort=cohort,phase=phase,uid=1000,samples=16,iterations=1024,warmups=32,product=False,admission=False),'Changed origin')
 origin=origins[0]
 require(all(type(origin[k]) is int for k in ('cohort','uid','samples','iterations','warmups')) and origin['product'] is False and origin['admission'] is False,'Changed origin types')
 rows=common.records(raw,'HC074-FILESYSTEM-COST ');require(len(rows)==448,'Incomplete/repeated work')
 for index,row in enumerate(rows):
  case,tail=divmod(index,64);sample,slot=divmod(tail,4)
  require(row['case']==CASES[case] and row['phase']==phase,'Changed case/phase')
  opens,fs,identity=(0,1,0) if case<2 else (4,4,0) if case==2 else (1,1,1)
  expected=[('cohort',cohort),('sample',sample),('slot',slot),('calls',1024),('opens_per_call',opens),('filesystem_checks',fs),('identity_checks',identity)]
  require(all(type(row[k]) is int and row[k]==v for k,v in expected),'Changed work/types')
  require(row['variant']==ARMS[common.ORDERS[(sample+cohort)%8][slot]],'Changed order')
  require(row['product'] is False and row['admission'] is False,'Changed scope')
  require(all(type(row[k]) is int and row[k]>0 for k in ('cpu_ns','wall_ns')),'Invalid clock')
 return rows
def review(cohorts,phase):
 require(phase in ('prototype','installed') and len(cohorts)==8,'Changed phase/process count')
 parsed=[parse(raw,i,phase) for i,raw in enumerate(cohorts)];cases={}
 for case in CASES:
  estimates={name:[] for name in ARMS[1:]};cross=[];times=[]
  for rows in parsed:
   rows=[r for r in rows if r['case']==case];pairs={name:[] for name in ARMS[1:]};checks=[];elapsed=[]
   for start in range(0,64,4):
    cpu={r['variant']:r['cpu_ns'] for r in rows[start:start+4]}
    for name in pairs:pairs[name].append(math.log(cpu[name]/cpu['reference']))
    checks.append(math.log(cpu['runtime']/cpu['prototype']));elapsed.append(cpu['reference']/1024)
   for name in pairs:estimates[name].append(statistics.median(pairs[name]))
   cross.append(statistics.median(checks));times.append(statistics.median(elapsed))
  ratios={name:common.interval(values) for name,values in estimates.items()};calibration=common.interval(cross)
  controls=common.inside(ratios['reference-control']) and common.inside(ratios['runtime'] if phase=='prototype' else calibration)
  candidate=ratios['prototype' if phase=='prototype' else 'runtime']
  cases[case]=dict(ratios=ratios,runtime_to_prototype=calibration,process_log_estimates=estimates,
   reference_cpu_ns_per_call=statistics.median(times),controls_passed=controls,cpu_screen_passed=controls and candidate['upper']<=0.98)
 return dict(phase=phase,cases=cases,cpu_screen_passed=all(c['cpu_screen_passed'] for c in cases.values()),
  original_checks_and_deadlines_unchanged=True,full_preparation_improvement_proven=False,product=False,admission=False)
