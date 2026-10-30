"""Current live diagnostic overlays, exact original inputs; never archived code."""
import json
from pathlib import Path
import shutil
import sys
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-frame-074'));import recipe as p
c=p.c
PINS={
 'tools/chitchat-inplace-074/p_count.rs':'07da8ffd0eea4802f31b675e75e37e9714be1ac7f25f9939a91f90e5617096e7',
 'tools/chitchat-inplace-074/d_budget.rs':'d75a7b8fd9a9e231631153d5681320f59038ff51f0c875c2a59d19c21192272d',
 'tools/chitchat-borrowed-074/native.py':'25be974c0e32e228076ede2f1150913235c26c504c26a2d608b4d2e59409521c',
 'tools/chitchat-call-context-074/parallel_source.py':'89278fa755e744dc17d4c0192c32960c74ca2e8f487fe561d7744b1d00fe26fb',
 'tools/chitchat-inplace-074/allocation_tests_portable.rs':'ab5cf5e12f5dfbb956ebcee59f8af554cf3030544eba07bd2e928ad2d0b109df',
 'tools/chitchat-native-cost-074/native_tests.rs':'ddcd1ac9310f5da7859ae7c411d18b62b7b966175cfb6c37d636e0f2ac7dbe92',
 'tools/chitchat-stateful-074/stateful.rs':'ec9520622675819b446ae22e8ff1c9913a1ec52214f54b8372ef61c7904248a2',
 'tools/chitchat-caller-074/caller.rs':'235499f1627ab07a176f1b6a8700b1dbf5625340bd20fa2a2cc7ec4ee78f21f4',
 'tools/chitchat-caller-074/cases-r2.json':'4a3308a82d014fb58df808a9f3984922537a80c02219610cdf5ee6fa1ec4a1f9'}

def held():
    for n,h in PINS.items():c.need(c.sha((ROOT/n).read_bytes())==h,'unchanged live qualified source/input: '+n)
    return PINS

def change(raw,before,after):
    c.need(raw.count(before)==1,'one diagnostic anchor: '+before[:80]);return raw.replace(before,after)

def span(text,marker):
    c.need(text.count(marker)==1,'one exact function')
    start=text.index(marker);opened=text.index('{',start);depth=1
    for end in range(opened+1,len(text)):
        depth+=(text[end]=='{')-(text[end]=='}')
        if depth==0:return start,end+1
    raise ValueError('unclosed source function')

def derive_p(folder):
    """Existing P count/budget algorithm, shared new dependency lock; control only."""
    path=folder/'src/serialize.rs';text=path.read_text()
    left,right=span(text,'    pub fn serialized_len_upperbound_after<')
    predictor='''    #[cfg(test)]
    pub fn serialized_len_upperbound_after<S:Serializable+?Sized>(&self,item:&S)->usize {
        let n=item.serialized_len();assert!(n>0);let total=self.uncompressed_block.len()+n;
        let old=self.output.len()+total+if total>self.block_threshold {7} else {4};
        if n>u16::MAX as usize || total<=2*self.block_threshold {return old;}
        let mut saved=self.clone();saved.append(item);old.max(saved.finish().len())
    }'''
    text=text[:left]+predictor+text[right:]
    left,right=span(text,'    pub(crate) fn try_append_borrowed<')
    # Remove the preceding cfg(test) so the existing P production admission is real.
    preceding=text.rfind('#[cfg(test)]',0,left)
    c.need(text[preceding+len('#[cfg(test)]'):left].strip()=='impl CompressedStreamWriter {','only test adapter impl attribute')
    text=text[:preceding]+text[preceding:].replace('#[cfg(test)]','',1)
    left,right=span(text,'    pub(crate) fn try_append_borrowed<')
    text=text[:left]+'''    pub(crate) fn try_append_borrowed<S:Serializable+?Sized>(&mut self,item:&S,budget:usize)->bool {
        if !self.fits_budget_after(item,budget) {return false;}self.append(item);true
    }'''+text[right:]
    count=(ROOT/'tools/chitchat-inplace-074/p_count.rs').read_text()
    count=change(count,'            let size = zstd::bulk::compress_to_buffer(',
      '            #[cfg(test)]\n            stateful_review::record();\n            #[cfg(test)]\n            crate::delta::cost_review::actual_caller::record_compression();\n            let size = zstd::bulk::compress_to_buffer(')
    budget=(ROOT/'tools/chitchat-inplace-074/d_budget.rs').read_text()
    budget=change(budget,'self.serialized_len_upperbound_after(item) <= budget','self.counted_upperbound_after(item) <= budget')
    text+='\n'+count+'\n'+budget;path.write_text(text,encoding='utf8',newline='\n')
    path=folder/'src/delta.rs';text=path.read_text()
    start=text.index('        if self\n            .compressed_stream_writer\n            .serialized_len_upperbound_after(&delta_op)')
    end=text.index('        assert!(self.delta_builder.apply_op(delta_op).is_ok());',start)
    text=text[:start]+'''        if !self.compressed_stream_writer.try_append_borrowed(&delta_op,self.mtu) {return false;}
'''+text[end:];path.write_text(text,encoding='utf8',newline='\n')

def augment_observer(text):
    text=change(text,'    ACTIVE.with(|active| active.set(true));',
      '    C_FREE_CHECKS.with(|v|v.set(0));\n    ACTIVE.with(|active| active.set(true));')
    text=change(text,'    let mut s = DeltaSerializer::with_mtu(mtu);',
      '    let mut s = DeltaSerializer::with_mtu(mtu);\n    c_free_checkpoint();')
    text=change(text,'    assert!(s.try_add_node(node(), 0, 0));',
      '    assert!(s.try_add_node(node(), 0, 0));\n    c_free_checkpoint();')
    text=change(text,'    (accepted, s.finish().serialize_to_vec())',
      '    c_free_checkpoint();\n    let wire=s.finish().serialize_to_vec();\n    c_free_checkpoint();\n    (accepted,wire)')
    text=change(text,'json!({"rust":rust,"native":native,"actual_calls":actual_calls})',
      'json!({"rust":rust,"native":native,"actual_calls":actual_calls,"c_free_checkpoints":c_free_checks()})')
    return text+'\n'+(HERE/'checkpoints.rs').read_text()

def augment_stateful(text):
    text=change(text,'    wire_len: usize,','    wire_len: usize,\n    finish_capacity:usize,')
    text=change(text,'        wire_len: 0,','        wire_len: 0,\n        finish_capacity:0,')
    text=change(text,'    let mut w = CompressedStreamWriter::with_block_threshold(case.threshold);\n    let mut trace',
      '    let mut w = CompressedStreamWriter::with_block_threshold(case.threshold);\n    crate::delta::cost_review::c_free_checkpoint();\n    let mut trace')
    text=change(text,'        trace.states[i + 1] = inventory(&w);',
      '        trace.states[i + 1] = inventory(&w);\n        crate::delta::cost_review::c_free_checkpoint();\n        assert!(w.compression_context.is_none());')
    text=change(text,'    let wire = black_box(w.finish());',
      '    let wire = black_box(w.finish());\n    crate::delta::cost_review::c_free_checkpoint();\n    trace.finish_capacity=wire.capacity();')
    text=change(text,'"compressions":trace.calls,"wire":wire_hex(&trace),',
      '"compressions":trace.calls,"wire":wire_hex(&trace),"finish_capacity":trace.finish_capacity,')
    return text

def augment_caller(text):
    text=change(text,'    wire_len: usize,','    wire_len: usize,\n    finish_capacity:usize,')
    text=change(text,'        wire_len: 0,','        wire_len: 0,\n        finish_capacity:0,')
    text=change(text,'    let mut s = DeltaSerializer::with_mtu(black_box(case.mtu));',
      '    let mut s = DeltaSerializer::with_mtu(black_box(case.mtu));\n    c_free_checkpoint();')
    text=change(text,'        trace.states[trace.attempted] = inventory(&s);',
      '        trace.states[trace.attempted] = inventory(&s);\n        c_free_checkpoint();\n        assert!(!s.compressed_stream_writer.has_compression_context());')
    text=change(text,'    let wire = black_box(delta.serialize_to_vec());',
      '    let wire = black_box(delta.serialize_to_vec());\n    c_free_checkpoint();\n    trace.finish_capacity=wire.capacity();')
    text=change(text,'"compressions":trace.calls,"paths":trace.paths,"wire":hex(&trace)',
      '"compressions":trace.calls,"paths":trace.paths,"wire":hex(&trace),"finish_capacity":trace.finish_capacity')
    return text

def prepare(core):
    import run as windows
    held();c.need(json.loads((p.BASE/'checkpoint.json').read_bytes())['checkpoint_pass'],'positive first checkpoint');out=core.INPUT;out.mkdir(exist_ok=False)
    for mode in ('check-G','clippy-G','compat-G','old0-G','old1-G','check-G-188','clippy-G-188','compat-G-188'):
        c.need(json.loads((windows.phase(mode)/'result.json').read_bytes())['complete'],'completed Windows gates before GNU resources')
    observer=(ROOT/'tools/chitchat-inplace-074/allocation_tests_portable.rs').read_text()
    for before,after in [('unsafe { System.alloc(layout) }','tag_system(|| unsafe { System.alloc(layout) })'),
       ('unsafe { System.alloc_zeroed(layout) }','tag_system(|| unsafe { System.alloc_zeroed(layout) })'),
       ('unsafe { System.dealloc(ptr, layout) };','tag_system(|| unsafe { System.dealloc(ptr, layout) });'),
       ('unsafe { System.realloc(ptr, layout, size) }','tag_system(|| unsafe { System.realloc(ptr, layout, size) })')]:
        observer=change(observer,before,after)
    observer+='\n'+(ROOT/'tools/chitchat-native-cost-074/native_tests.rs').read_text()
    observer=augment_observer(observer)
    observer+='''
pub(crate) fn stateful_capture<T>(operation:impl FnOnce()->T)->(T,serde_json::Value) {
    native_begin();let (result,rust)=measured(operation);let native=native_finish();
    assert_eq!(rust.live,0);(result,json!({"rust":rust,"native":native,"c_free_checkpoints":c_free_checks()}))
}
#[path="caller.rs"]
pub(crate) mod actual_caller;
'''
    stateful=(ROOT/'tools/chitchat-stateful-074/stateful.rs').read_text()
    stateful=change(stateful,'if STATEFUL_VARIANT != "Q"','if STATEFUL_VARIANT != "Q" && STATEFUL_VARIANT != "G"')
    stateful+='''
impl BorrowedItem for Blob {
    fn visit(&self,sink:&mut impl FnMut(&[u8])) { sink(&self.0); }
}
'''
    stateful=augment_stateful(stateful)
    for v in ('B','P','G'):
        folder=out/'fixtures'/v;shutil.copytree(p.BASE/'fixtures'/('B' if v=='P' else v),folder)
        if v=='P':derive_p(folder)
        src=folder/'src'
        for name,body in [('observer.rs',observer),('stateful.rs',stateful),
                         ('caller.rs',augment_caller((ROOT/'tools/chitchat-caller-074/caller.rs').read_text()))]:
            (src/name).write_text(body,encoding='utf8',newline='\n')
        (src/'cases-r2.json').write_bytes((ROOT/'tools/chitchat-caller-074/cases-r2.json').read_bytes())
        path=src/'delta.rs';text=path.read_text()
        text+='\n#[cfg(test)]\n#[path="observer.rs"]\npub(crate) mod cost_review;\n#[cfg(test)]\nconst CALLER_VARIANT:&str="'+v+'";\n'
        path.write_text(text,encoding='utf8',newline='\n')
        path=src/'serialize.rs';text=path.read_text()
        text=change(text,'    call_context_review::after_context_ready(context.is_some());',
          '    call_context_review::after_context_ready(context.is_some());\n    #[cfg(test)]\n    stateful_review::record();\n    #[cfg(test)]\n    crate::delta::cost_review::actual_caller::record_compression();')
        # Only test counters enter the candidate's fixed branch structure.
        if v=='G':
            text=change(text,'        if self.legacy_bound(n) > budget {','        #[cfg(test)]\n        inplace_review::record(if self.legacy_bound(n)>budget {2} else if self.output.len()+self.uncompressed_block.len()+n+3*(self.uncompressed_block.len()+n).div_ceil(self.block_threshold)<budget {3} else {1});\n        if self.legacy_bound(n) > budget {')
        elif v=='B':
            text=change(text,'        let new_item_serialized_len = item.serialized_len();','        #[cfg(test)]\n        inplace_review::record(0);\n        let new_item_serialized_len = item.serialized_len();')
        text+='''
#[cfg(test)]
impl CompressedStreamWriter {
    pub(crate) fn has_compression_context(&self)->bool {self.compression_context.is_some()}
    fn stateful_cache(&self)->Option<usize> {self.tail_cache}
    fn stateful_append<I:BorrowedItem+?Sized>(&mut self,item:&I,budget:usize)->bool {self.try_append_borrowed(item,budget)}
    pub(crate) fn cost_inventory(&self)->([usize;3],usize) {
        ([self.output.len(),self.uncompressed_block.len(),self.compressed_block.len()],self.block_threshold)
    }
    pub(crate) fn caller_inventory(&self)->([usize;3],[usize;3],Option<usize>) {
        ([self.output.len(),self.uncompressed_block.len(),self.compressed_block.len()],
         [self.output.capacity(),self.uncompressed_block.capacity(),self.compressed_block.capacity()],self.tail_cache)
    }
}
#[cfg(test)]
pub(crate) mod inplace_review {
    use std::cell::Cell;
    thread_local! {static COUNTS:Cell<[u64;4]>=const{Cell::new([0;4])};}
    pub(crate) fn record(i:usize) {COUNTS.with(|v|{let mut a=v.get();a[i]+=1;v.set(a)});}
    pub(crate) fn reset() {COUNTS.with(|v|v.set([0;4]));}
    pub(crate) fn take()->[u64;4] {COUNTS.with(Cell::get)}
}
#[cfg(test)]
#[path="stateful.rs"]
mod stateful_review;
'''
        text+='\n#[cfg(test)]\nconst STATEFUL_VARIANT:&str="'+v+'";\n'
        path.write_text(text,encoding='utf8',newline='\n')
    old=ROOT/'tools/chitchat-native-cost-074';prior=ROOT/'tools/chitchat-call-context-074'
    for name in ('wrap.h','control.c'):(out/name).write_bytes((old/name).read_bytes())
    (out/'wrap.c').write_bytes((old/'wrap.c').read_bytes()+b'\n'+(prior/'peek.c').read_bytes())
    (out/'peek_control.c').write_bytes((prior/'peek_control.c').read_bytes())
    c.write(out,'preparation.json',dict(source=core.source(),fixtures={v:c.tree(p.BASE/'fixtures'/v) for v in ('B','G')},
      pins=PINS,variants=['B','P','G'],G_production_functional_changes=False,P_control_algorithm="p_count/d_budget",root_adopted=False,resources_qualified=False))
    c.write(p.BASE,core.FRAME.name,c.tree(out))
