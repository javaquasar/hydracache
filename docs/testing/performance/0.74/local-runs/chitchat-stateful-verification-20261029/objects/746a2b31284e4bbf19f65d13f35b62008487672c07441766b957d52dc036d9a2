"""Restore actual B/P/Q; append only visibility/diagnostic observer surfaces."""
import json
import os
import re
import common as c

INDEX='853f8691d7e9e1f0d2c1143908fddaefdb36e0fd24a507cbf7ac4424a3c61a71'


def packet():
    folder=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-commit-20261029'
    raw=(folder/'index.json').read_bytes();c.need(c.sha(raw)==INDEX,'sealed Q source packet')
    members=json.loads(raw)['members']
    def get(name):
        row=members[name];body=(folder/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'exact source member '+name);return body
    return members,get


def change(text,before,after):
    c.need(text.count(before)==1,'one patch anchor '+before[:80]);return text.replace(before,after)


def main():
    members,get=packet();q='capture/chitchat-commit-Q-20261029-r2/';p='capture/chitchat-commit-P-20261029-r2/'
    (c.BASE/'input').mkdir();observer=get(q+'observer.rs').decode()
    observer+='''
pub(crate) fn stateful_capture<T>(operation: impl FnOnce() -> T) -> (T, serde_json::Value) {
    native_begin();
    let (result, rust) = measured(operation);
    let native = native_finish();
    assert_eq!(rust.live, 0);
    (result, json!({"rust":rust,"native":native}))
}
'''
    (c.BASE/'observer.rs').write_text(observer,encoding='utf8',newline='\n')
    for variant in ('B','P','Q'):
        source=p if variant=='P' else q;prefix=source+'fixtures/'+variant+'/'
        folder=c.BASE/'fixtures'/variant
        for name in members:
            if name.startswith(prefix):
                dest=folder/name[len(prefix):];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(get(name))
        path=folder/'src/delta.rs';text=path.read_text(encoding='utf8')
        relative=os.path.relpath(c.BASE/'observer.rs',path.parent).replace('\\','/')
        text=re.sub(r'(?<=#\[path = ")[^"\n]+(?="\]\nmod cost_review;)',relative,text)
        text=change(text,'mod cost_review;','pub(crate) mod cost_review;');path.write_text(text,encoding='utf8',newline='\n')
        path=folder/'src/serialize.rs';text=path.read_text(encoding='utf8')
        text=change(text,'    q_review::record();','    q_review::record();\n    #[cfg(test)]\n    stateful_review::record();')
        cache='self.tail_cache' if variant=='Q' else 'None'
        append='if self.serialized_len_upperbound_after(item)>budget {return false;} self.append(item);true' if variant=='B' else 'self.try_append_with_budget(item,budget)'
        text+='\n#[cfg(test)]\nimpl CompressedStreamWriter {\nfn stateful_cache(&self)->Option<usize> {'+cache+'}\n'
        text+='fn stateful_append<S:Serializable+?Sized>(&mut self,item:&S,budget:usize)->bool {'+append+'}\n}\n'
        relative=os.path.relpath(c.TOOLS/'stateful.rs',path.parent).replace('\\','/')
        text+='\n#[cfg(test)]\nconst STATEFUL_VARIANT:&str="'+variant+'";\n#[cfg(test)]\n#[path="'+relative+'"]\nmod stateful_review;\n'
        path.write_text(text,encoding='utf8',newline='\n')
        catalog=json.loads(get(source+'build-'+variant+'/native/catalog.json'))
        added=['serialize::stateful_review::'+n for n in ('stateful_semantics_control','stateful_capacity_control','stateful_observations')]
        c.write(c.BASE/'input','catalog-'+variant+'.json',sorted(catalog+added))
    for name in members:
        if name.startswith(q+'input/') and '/catalog-' not in name:
            dest=c.BASE/name[len(q):];dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(get(name))
    names=set(c.source())
    old_frame=json.loads(get(q+'frame.json'))
    for name,digest in old_frame.items():
        if name.startswith('tools/') and name.endswith('.rs'):
            c.need(c.sha((c.ROOT/name).read_bytes())==digest,'unchanged original Rust overlay '+name);names.add(name)
    for folder in (c.BASE/'fixtures',c.BASE/'input'):
        names|={p.relative_to(c.ROOT).as_posix() for p in folder.rglob('*') if p.is_file()}
    names.add((c.BASE/'observer.rs').relative_to(c.ROOT).as_posix());c.write(c.BASE,'input-names.json',sorted(names))
    c.write(c.BASE,'derivation.json',dict(predecessor=INDEX,variants=['B','P','Q'],production_methods_changed=False,root_adopted=False))


if __name__=='__main__':main()
