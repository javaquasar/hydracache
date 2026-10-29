"""Derive independent fixtures; Q cannot be materialized before P's first refusals."""
import json
import os
import re
import common as c

PREDECESSOR='3550fa5293b3f61f3b6d06662e3090b17658a42c1ca87c97b32f0c20e6d6c489'
PREFIX='capture/chitchat-native-cost-opaque-20261029/'


def packet():
    folder=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-native-cost-20261029'
    raw=(folder/'index.json').read_bytes();c.need(c.sha(raw)==PREDECESSOR,'sealed predecessor')
    members=json.loads(raw)['members']
    def get(name):
        row=members[name];body=(folder/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed member '+name)
        return body
    return members,get


def replace(text,before,after):
    c.need(text.count(before)==1,'one exact patch anchor: '+before[:80])
    return text.replace(before,after)


def fixture(variant,members,get):
    source='P' if variant=='Q' else variant
    folder=c.BASE/'fixtures'/variant
    prefix=PREFIX+'fixtures/'+source+'/'
    for key in members:
        if key.startswith(prefix):
            path=folder/key[len(prefix):];path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(get(key))
    path=folder/'src/delta.rs';text=path.read_text(encoding='utf8')
    relative=os.path.relpath(c.BASE/'observer.rs',path.parent).replace('\\','/')
    text=re.sub(r'(?<=#\[path = ")[^"\n]+(?="\]\nmod cost_review;)',relative,text)
    if variant=='Q':
        text=replace(text,'.fits_budget_after(&delta_op, self.mtu)','.try_append_with_budget(&delta_op, self.mtu)')
        text=replace(text,'        self.compressed_stream_writer.append(&delta_op);\n','')
    path.write_text(text,encoding='utf8',newline='\n')
    path=folder/'src/serialize.rs';text=path.read_text(encoding='utf8')
    if variant=='Q':
        text=replace(text,'    block_threshold: usize,\n','    block_threshold: usize,\n    tail_cache: Option<usize>,\n')
        text=replace(text,'            block_threshold,\n','            block_threshold,\n            tail_cache: None,\n')
        text=replace(text,'        item.serialize(&mut self.uncompressed_block);\n','        self.tail_cache = None;\n        item.serialize(&mut self.uncompressed_block);\n')
        anchor='        let num_bytes_to_compress = self.uncompressed_block.len().min(self.block_threshold);'
        cached='''        if let Some(n) = self.tail_cache.take() {
            assert!(self.uncompressed_block.len() <= self.block_threshold);
            BlockType::Compressed.serialize(&mut self.output);
            u16::try_from(n).unwrap().serialize(&mut self.output);
            self.output.extend_from_slice(&self.compressed_block[..n]);
            self.uncompressed_block.clear();
            return;
        }
'''
        text=replace(text,anchor,cached+anchor)
        for name in ('pub(crate) fn fits_budget_after','fn counted_upperbound_after'):
            text=replace(text,'    '+name,'    #[cfg(test)]\n    '+name)
        text+='\n'+(c.TOOLS/'q_append.rs').read_text(encoding='utf8')
        text+='\n#[cfg(test)]\nimpl CompressedStreamWriter { fn q_cache(&self) -> Option<usize> { self.tail_cache } }\n'
    elif variant=='P':
        text+='''
#[cfg(test)]
impl CompressedStreamWriter {
    fn q_cache(&self) -> Option<usize> { None }
    fn try_append_with_budget<S: Serializable + ?Sized>(&mut self, item: &S, budget: usize) -> bool {
        if !self.fits_budget_after(item, budget) { return false; }
        self.append(item); true
    }
}
'''
    c.need(text.count('zstd::bulk::compress_to_buffer(')==(4 if variant=='Q' else 2 if variant=='P' else 1),'exact compression call-site count')
    text=text.replace('zstd::bulk::compress_to_buffer(','q_compress_block(')
    text+='''
fn q_compress_block(source: &[u8], destination: &mut [u8], level: i32) -> std::io::Result<usize> {
    #[cfg(test)]
    q_review::record();
    zstd::bulk::compress_to_buffer(source, destination, level)
}
'''
    if variant in ('P','Q'):
        relative=os.path.relpath(c.TOOLS/'q_tests.rs',path.parent).replace('\\','/')
        text+='\n#[cfg(test)]\n#[path = "'+relative+'"]\nmod q_review;\n'
    else:
        text+='\n#[cfg(test)]\nmod q_review { pub(crate) fn record() {} }\n'
    path.write_text(text,encoding='utf8',newline='\n')


def main():
    if c.PROFILE=='Q':
        previous=c.ROOT/'target/chitchat-commit-P-20261029-r2/regression-P'
        c.need(json.loads((previous/'result.json').read_bytes())['complete'],'recorded P baseline before Q derivation')
        result=json.loads((previous/'native/regressions.json').read_bytes())
        c.need(result['passed']==4 and result['failed']==3 and result['expected_refusals'],'exact first baseline outcome')
    members,get=packet();(c.BASE/'input').mkdir()
    (c.BASE/'observer.rs').write_bytes(get(PREFIX+'observer.rs'))
    variants=('P',) if c.PROFILE=='P' else ('B','Q')
    for variant in variants:fixture(variant,members,get)
    for key in members:
        if key.startswith(PREFIX+'input/'):
            name=key[len(PREFIX):];path=c.BASE/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(get(key))
    c.need(c.sha((c.BASE/'input/reuse-object.json').read_bytes())==c.sha(get(PREFIX+'input/reuse-object.json')),'same qualified native object receipt')
    for variant in variants:
        catalog=json.loads(get(PREFIX+'build-'+('P' if variant=='Q' else variant)+'/native/catalog.json'))
        if variant in ('P','Q'):
            names=re.findall(r'#\[test\]\s*fn (\w+)\(', (c.TOOLS/'q_tests.rs').read_text(encoding='utf8'))
            c.need(len(names)==7,'frozen seven regression tests');catalog+=['serialize::q_review::'+n for n in names]
        path=c.BASE/'input'/('catalog-'+variant+'.json')
        path.write_text(json.dumps(sorted(catalog),indent=2)+'\n',encoding='utf8',newline='\n')
    # Include every external overlay referred to by restored fixture source.
    inherited=json.loads(get(PREFIX+'frame.json'))
    names=set(c.source())
    for name in inherited:
        if name.startswith('tools/') and not name.startswith('tools/chitchat-native-cost-074/'):
            c.need(c.sha((c.ROOT/name).read_bytes())==inherited[name],'unchanged external overlay '+name);names.add(name)
    for folder in (c.BASE/'fixtures',c.BASE/'input'):
        names|={p.relative_to(c.ROOT).as_posix() for p in folder.rglob('*') if p.is_file()}
    names.add((c.BASE/'observer.rs').relative_to(c.ROOT).as_posix())
    c.write(c.BASE,'input-names.json',sorted(names))
    c.write(c.BASE,'derivation.json',dict(predecessor=PREDECESSOR,profile=c.PROFILE,variants=variants,root_adopted=False))


if __name__=='__main__':main()
