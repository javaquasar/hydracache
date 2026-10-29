"""Seal all first outcomes and four complete peers; exclude compiled caches."""
import json
import os
import subprocess
import analyze as a
import common as c

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-compat-20261029'


def main():
    result=a.reconcile();c.need(result==json.loads((c.BASE/'analysis/result.json').read_bytes()),'independent raw accounting')
    for mode in ('controls-r1','format','performance-contract'):
        c.need(json.loads((c.BASE/'final'/mode/'result.json').read_bytes())['complete'],'first complete corrected source gate')
    c.need(not json.loads((c.BASE/'final/controls/result.json').read_bytes())['complete'],'first offline import refusal retained')
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        c.need(path.is_file() and not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular owned capture')
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'unique logical member');members[name]=dict(sha256=digest,bytes=len(body))
    for context in (a.R0,a.R1):
        base=c.ROOT/'target'/context
        for folder,dirs,names in os.walk(base,followlinks=False):
            path=c.Path(folder)
            for name in dirs:c.need(not (path/name).is_symlink() and not getattr(path/name,'is_junction',lambda:False)(),'regular owned directory')
            if path==base:dirs[:]=[d for d in dirs if d not in ('windows-targets','cargo-home')]
            for name in sorted(names):add('capture/'+context+'/'+(path/name).relative_to(base).as_posix(),path/name)
        add('capture/'+context+'/cargo-home/config.toml',base/'cargo-home/config.toml')
    for name in c.source():add('source/'+name,c.ROOT/name)
    index=dict(schema='hc074-chitchat-compat-v1',members=members,
        predecessor_index_sha256='874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56',
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        table_sha256=c.TABLE_SHA,compatibility_gate_pass=True,test_executions=340,unique_tests=85,
        first_outer_timeouts_retained=2,old_inner_results_promoted=False,compiled_images_excluded=True,
        original_zip_inputs_retained=True,archived_code_executed=False,native_C_qualified=False,
        CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed member verification')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
