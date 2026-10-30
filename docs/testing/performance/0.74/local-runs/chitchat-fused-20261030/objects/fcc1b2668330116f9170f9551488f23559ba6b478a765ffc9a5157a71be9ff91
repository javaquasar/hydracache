"""Offline exhaustive source-bound proof, no product/live/measurement execution."""
import json

M=65535

def review():
    encoded=[]; workspace=[]
    for t in range(1,M+1):
        n=t+M
        assert t+(M+t-1)//t<=M+1
        wire=n+3*((n+t-1)//t)+1
        storage=2*t+wire-1
        assert storage<=262146
        encoded.append((wire,t));workspace.append((storage,t))
    assert max(encoded)==(262145,1)
    assert max(v for v,_ in workspace)==262146
    return dict(thresholds=M,item_max=M,pending_max="threshold",encoded_max=262145,
      storage_envelope=262146,storage_max_thresholds=[t for v,t in workspace if v==262146],
      mtu_upper_limit_added=False,stack_RSS_qualified=False,root_adopted=False)

if __name__=='__main__':print(json.dumps(review(),sort_keys=True))
