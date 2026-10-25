//! Bounded diagnostic event model. It observes no source, clock or process.
use serde_json::{json, Value};

pub const LIMIT: u64 = 4096;
pub const OPERATIONS: [&str; 4] = ["git", "output", "decode", "read"];
struct Frame {
    id: u64,
    parent: u64,
    operation: &'static str,
    args: Vec<String>,
    start_ns: u64,
}
pub struct Buffer {
    next: u64,
    stack: Vec<Frame>,
    records: Vec<Value>,
    valid: bool,
}
impl Buffer {
    pub fn new() -> Self {
        Self {
            next: 0,
            stack: Vec::new(),
            records: Vec::new(),
            valid: true,
        }
    }
    pub fn begin(
        &mut self,
        operation: &'static str,
        args: &[&str],
        start_ns: u64,
    ) -> Result<u64, &'static str> {
        if !OPERATIONS.contains(&operation)
            || args.iter().map(|s| s.len()).sum::<usize>() > 4096
            || args.len() > 64
            || self.stack.len() >= 4
            || self.next >= LIMIT
        {
            self.valid = false;
            return Err("helper-profile-input-or-record-budget");
        }
        self.next += 1;
        let id = self.next;
        let parent = self.stack.last().map_or(0, |frame| frame.id);
        self.stack.push(Frame {
            id,
            parent,
            operation,
            args: args.iter().map(|s| (*s).to_owned()).collect(),
            start_ns,
        });
        Ok(id)
    }
    pub fn finish(
        &mut self,
        id: u64,
        end_ns: u64,
        output: Option<(usize, usize)>,
        unwinding: bool,
    ) {
        if !self
            .stack
            .last()
            .is_some_and(|frame| frame.id == id && end_ns >= frame.start_ns)
        {
            self.valid = false;
            return;
        }
        let frame = self.stack.pop().unwrap();
        self.records.push(
            json!({"id":id,"parent":frame.parent,"operation":frame.operation,
            "arguments":frame.args,"start_ns":frame.start_ns,"end_ns":end_ns,
            "elapsed_ns":end_ns-frame.start_ns,"returned":output.is_some(),"unwinding":unwinding,
            "output_bytes":output.map(|v|v.0),"stderr_bytes":output.map(|v|v.1)}),
        );
    }
    pub fn report(&self, case: &str, thread: &str) -> Value {
        json!({"schema":"hc074-helper-trace-v1","case":case,"thread":thread,
            "valid":self.valid && self.stack.is_empty() && self.records.len() as u64 == self.next,
            "started":self.next,"records":self.records,
            "nested_intervals":true,"product":false,"package_replay":false})
    }
}
