"""Fixed independent workload producer; no observed-result-dependent boundaries."""
from pathlib import Path
import json
import sys


def ascii95(n):
    seed=0x0741028;out=[];mask=(1<<64)-1
    for _ in range(n):
        seed^=(seed<<13)&mask;seed^=seed>>7;seed^=(seed<<17)&mask;out.append(chr(32+seed%95))
    return ''.join(out)


def table():
    result=[]
    node=lambda name,port:dict(kind='node',id=name,port=port)
    kv=lambda key,value,version:dict(kind='kv',key=key,value=value,version=version)
    for mtu in (100,198,1000,16384,40000,65507):
        first=node('node',7280)
        small=[kv('k'+str(i),ascii95(16),i+1) for i in range(8)]
        large=kv('k',ascii95(mtu-62),1)
        families=[
            ('small-distinct',[first,*small]),
            ('large-then-small',[first,large,*[kv('k'+str(i),'r'*16,i+2) for i in range(4)]]),
            ('small-then-boundary',[first,*small[:2],kv('large',ascii95(mtu-62),3)]),
            ('rejecting-boundary',[first,kv('reject',ascii95(mtu),1),kv('never','r',2)]),
            ('multiple-nodes',[first,dict(kind='max',version=2),node('node-b',7281),kv('b',ascii95(64),1),node('node-c',7282),dict(kind='max',version=2)]),
        ]
        result.extend(dict(name=f'{mtu}/{name}',mtu=mtu,requests=requests) for name,requests in families)
    return dict(seed=0x0741028,cases=result)


if __name__=='__main__':
    assert sys.argv[1:]==['write']
    with Path(__file__).with_name('cases-r2.json').open('x',encoding='utf8',newline='\n') as out:json.dump(table(),out,indent=2,sort_keys=True);out.write('\n')
