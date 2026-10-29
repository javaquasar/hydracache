"""Seal both source contexts and first refusals; no archived code execution."""
import json
import os
import subprocess
import common as c
import review_refusal

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cost-20261029'


def main():
    c.need(review_refusal.review()==json.loads((c.BASE/'diagnosis/result.json').read_bytes()),'exact first refusal analysis')
    for mode in ('controls-final','format','performance-contract'):
        c.need(json.loads((c.BASE/'final'/mode/'result.json').read_bytes())['complete'],'complete current offline checks')
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        c.need(path.is_file() and not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular owned source/capture')
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'unique logical member');members[name]=dict(sha256=digest,bytes=len(body))
    for context in ('chitchat-cost-20261029','chitchat-cost-20261029-r1'):
        base=c.ROOT/'target'/context
        for folder,dirs,names in os.walk(base,followlinks=False):
            folder=c.Path(folder)
            for name in dirs:c.need(not (folder/name).is_symlink() and not getattr(folder/name,'is_junction',lambda:False)(),'regular capture directory')
            if folder==base:dirs[:]=[n for n in dirs if n not in ('cargo-home','windows-targets')]
            for name in sorted(names):add('capture/'+context+'/'+(folder/name).relative_to(base).as_posix(),folder/name)
        add('capture/'+context+'/cargo-home/config.toml',base/'cargo-home/config.toml')
    for name in c.source():add('source/'+name,c.ROOT/name)
    index=dict(schema='hc074-chitchat-clean-cost-v1',members=members,
        actual_caller_index_sha256='874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56',
        compatibility_index_sha256='865f98e6b82240f06b71cf7b316b441766a6653947fa6d8899fa8540f2fb9bfc',
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        clock_control_pass=False,cohort_processes_dispatched=0,CPU_qualified=False,RSS_qualified=False,
        allocator_retention_qualified=False,generic_component_gate_pass=False,root_adopted=False,release_admitted=False,
        compiled_images_excluded=True,archived_code_executed=False,first_refusals_retained=True)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed member verification')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
