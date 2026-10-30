"""Immutable B0 control first; C1 changes only the private compressor lifetime."""
import json
from pathlib import Path
import shutil
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'chitchat-security-only-074'))
import scope as c

BASE=c.ROOT/'target/chitchat-borrowed-20261030'
HERE=Path(__file__).resolve().parent
OLD=b'''        match zstd::bulk::compress_to_buffer(
            &self.uncompressed_block[..num_bytes_to_compress],
            &mut self.compressed_block[..],
            0, // default compression level
        ) {'''
NEW=b'''        match compress_block_with_context(
            &mut self.compression_context,
            &self.uncompressed_block[..num_bytes_to_compress],
            &mut self.compressed_block[..],
        ) {'''
STATELESS=b'''fn compress_block_with_context(
    context: &mut Option<zstd::bulk::Compressor<'static>>,
    source: &[u8], destination: &mut [u8],
) -> std::io::Result<usize> {
    let _ = context;
    zstd::bulk::compress_to_buffer(source, destination, 0)
}
'''
OWNED=b'''fn compress_block_with_context(
    context: &mut Option<zstd::bulk::Compressor<'static>>,
    source: &[u8], destination: &mut [u8],
) -> std::io::Result<usize> {
    if context.is_none() {
        *context = Some(zstd::bulk::Compressor::new(0)?);
    }
    context.as_mut().unwrap().compress_to_buffer(source, destination)
}
'''

def baseline():
    BASE.mkdir(exist_ok=False);out=BASE/'preparation';out.mkdir()
    origin=c.BASE/'fixtures/U';snapshot=c.tree(origin)
    folder=BASE/'fixtures/B0';shutil.copytree(origin,folder)
    path=folder/'src/serialize.rs';body=path.read_bytes()
    c.need(body.count(OLD)==1,'one exact original bulk call')
    body=body.replace(b'    block_threshold: usize,\n',b"    block_threshold: usize,\n    compression_context: Option<zstd::bulk::Compressor<'static>>,\n",1)
    body=body.replace(b'            block_threshold,\n',b'            block_threshold,\n            compression_context: None,\n',1)
    body=body.replace(OLD,NEW)+b'\n'+STATELESS+b'\n#[cfg(test)]\n#[path="context_control.rs"]\nmod borrowed_context_control;\n'
    path.write_bytes(body);(folder/'src/context_control.rs').write_bytes((HERE/'context_tests.rs').read_bytes())
    c.write(out,'baseline.json',dict(parent_fixture=snapshot,baseline=c.tree(folder),context_control_only=True,mtu_repaired=False,root_adopted=False))

def candidate():
    value=json.loads((BASE/'control-B0/result.json').read_bytes())
    c.need(value['complete'] and value['expected_failure'],'actual failing context-retention control precedes implementation')
    parent=BASE/'fixtures/B0';c.need(c.tree(parent)==json.loads((BASE/'preparation/baseline.json').read_bytes())['baseline'],'immutable baseline')
    folder=BASE/'fixtures/C1';shutil.copytree(parent,folder)
    path=folder/'src/serialize.rs';old=path.read_bytes();c.need(old.count(STATELESS)==1,'one exact native-context hypothesis')
    new=old.replace(STATELESS,OWNED);path.write_bytes(new)
    c.write(BASE/'preparation','candidate.json',dict(baseline_sha256=c.sha(old),candidate_sha256=c.sha(new),
      changed_function='compress_block_with_context',fixed_level=0,first_full_block_then_no_larger_blocks=True,
      new_vec=False,public_trait_changed=False,mtu_repaired=False,root_adopted=False))

if __name__=='__main__':{'baseline':baseline,'candidate':candidate}[sys.argv[1]]()
