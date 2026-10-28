"""Freeze resolved graphs and exact public cache archives for offline Linux."""
from pathlib import Path
import importlib.util,json,os,tomllib
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('owner_frozen_preparation',ROOT/'tools/dbus-original-owner-074/prepare.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
def main():
 out=r.BASE/'freeze';out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(Path(__file__).read_bytes())
 sources=json.loads((r.BASE/'prepare/result.json').read_bytes());p=r.p
 allowed={}
 for n in tomllib.loads((ROOT/'Cargo.lock').read_text(encoding='utf8'))['package']:
  if 'checksum' in n:allowed[(n['name'],n['version'])]=n['checksum']
 parent=ROOT/'target/dbus-fixed-channel-20261028/api-fixed/input/fd/Cargo.lock'
 for n in tomllib.loads(parent.read_text(encoding='utf8'))['package']:
  if 'checksum' in n:
   key=(n['name'],n['version']);p.require(key not in allowed or allowed[key]==n['checksum'],'no conflicting primary identity');allowed[key]=n['checksum']
 registry={};graphs={}
 for variant in ('baseline','fd'):
  resolution=json.loads((r.BASE/('resolve-'+variant)/'result.json').read_bytes());p.require(resolution['exit']==0 and r.files(r.BASE/'input'/variant)==resolution['files_after'],'exact resolved variant')
  nodes=tomllib.loads((r.BASE/'input'/variant/'Cargo.lock').read_text(encoding='utf8'))['package'];graphs[variant]=nodes
  for node in nodes:
   if 'checksum' not in node:continue
   key=(node['name'],node['version']);p.require(allowed.get(key)==node['checksum'],'registry identity outside already-held product/backport inputs: '+str(key))
   name=node['name']+'-'+node['version'];registry[name]=dict(name=name,sha256=node['checksum'])
 archives=r.BASE/'input/archives';archives.mkdir()
 cache=Path(os.environ['USERPROFILE'])/'.cargo/registry/cache'
 for name,row in sorted(registry.items()):
  matches=list(cache.glob('*/'+name+'.crate'));p.require(matches,'held registry archive missing: '+name)
  raw=matches[0].read_bytes();p.require(r.sha(raw)==row['sha256'],'primary cache checksum');(archives/(name+'.crate')).write_bytes(raw);row['bytes']=len(raw)
 for variant in ('baseline','fd'):
  path=r.BASE/'input'/variant/'.cargo/config.toml';path.parent.mkdir(exist_ok=True)
  path.write_text('[source.crates-io]\nreplace-with = "held-vendor"\n[source.held-vendor]\ndirectory = "'+r.NATIVE+'/vendor"\n',encoding='utf8',newline='\n')
 helpers=dict(json.loads((ROOT/'target/dbus-fixed-channel-20261028/api-fixed/prepare/result.json').read_bytes())['helpers'])
 r.write(out,'result.json',dict(files=r.files(r.BASE/'input'),root_inputs=sources['root_inputs'],helpers=helpers,registry=list(registry.values()),native_root=r.NATIVE,graphs=graphs,
  library_only=True,actual_original_frame=True,actual_receiver_binding=True,context_stubs=False,tests_executed=False,system_bus=False,root_changed=False,release_admitted=False))
 print(json.dumps(dict(frozen=True,registry_archives=len(registry),nodes={v:len(n) for v,n in graphs.items()})))
if __name__=='__main__':main()
