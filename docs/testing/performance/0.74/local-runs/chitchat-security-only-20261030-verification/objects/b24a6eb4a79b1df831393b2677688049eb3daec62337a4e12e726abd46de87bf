"""Independently reconcile retained raw outputs without running archived code."""
import json
import analyze as a
from retain import OUT
c=a.c

def validate(index,read_object):
    c.need(index['schema']=='hc074-chitchat-dependency-only-packet-v1' and index['archived_code_executed'] is False,'data-only packet')
    c.need(index['root_adopted'] is False and index['release_admitted'] is False,'no top-level admission promotion')
    files={}
    for name,row in index['members'].items():
        body=read_object(row['sha256']);c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole stored bytes')
        files[name]=body
    def read(name):return files[name] if name.startswith('root/') else files['capture/r4/'+name]
    result=a.analyze(read,lambda n:'capture/r4/'+n in files)
    c.need(result==index['result']==json.loads(files['analysis.json']),'independent whole-result replay')
    return result

if __name__=='__main__':
    index=json.loads((OUT/'index.json').read_bytes());result=validate(index,lambda h:(OUT/'objects'/h).read_bytes())
    print(json.dumps(dict(verified_members=len(index['members']),completed_peers=result['completed_peers'],wide_gate_passed=False,root_adopted=False)))
