//! Opt-in diagnostics only. No event allocation or clock read while disabled.
#[path = "helper_model.rs"]
pub mod model;
use std::cell::RefCell;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

struct Local {
    anchor: Instant,
    case: String,
    thread: String,
    buffer: model::Buffer,
}
impl Local {
    fn new() -> Self {
        let thread = std::thread::current();
        Self {
            anchor: Instant::now(),
            case: thread.name().unwrap_or("unnamed").to_owned(),
            thread: format!("{:?}", thread.id()),
            buffer: model::Buffer::new(),
        }
    }
    fn now(&self) -> u64 {
        self.anchor
            .elapsed()
            .as_nanos()
            .try_into()
            .expect("helper-profile-clock-budget")
    }
}
impl Drop for Local {
    fn drop(&mut self) {
        // A missing/partial case record is rejected by the external reader.
        // Do not replace an original unwind with a second stderr-write panic.
        let _ = writeln!(
            std::io::stderr().lock(),
            "HC074_HELPER_TRACE {}",
            self.buffer.report(&self.case, &self.thread)
        );
    }
}
thread_local! { static LOCAL: RefCell<Local> = RefCell::new(Local::new()); }
fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var_os("HC074_PRESERVATION_PROFILE") {
        None => false,
        Some(value) => {
            assert_eq!(value, "v1", "unknown helper diagnostic protocol");
            true
        }
    })
}
pub struct Scope {
    id: u64,
    output: Option<(usize, usize)>,
}
impl Drop for Scope {
    fn drop(&mut self) {
        LOCAL.with(|local| {
            let mut local = local.borrow_mut();
            let end = local.now();
            local
                .buffer
                .finish(self.id, end, self.output, std::thread::panicking());
        });
    }
}
fn begin(operation: &'static str, args: &[&str]) -> Option<Scope> {
    if !enabled() {
        return None;
    }
    Some(LOCAL.with(|local| {
        let mut local = local.borrow_mut();
        let now = local.now();
        let id = local
            .buffer
            .begin(operation, args, now)
            .expect("helper-profile-refused");
        Scope { id, output: None }
    }))
}
pub fn read(path: &str) -> Option<Scope> {
    begin("read", &[path])
}
pub fn git(args: &[&str]) -> Option<Scope> {
    begin("git", args)
}
pub fn output() -> Option<Scope> {
    begin("output", &[])
}
pub fn decode() -> Option<Scope> {
    begin("decode", &[])
}
pub fn finish(scope: Option<Scope>, output_bytes: usize, stderr_bytes: usize) {
    if let Some(mut scope) = scope {
        scope.output = Some((output_bytes, stderr_bytes));
    }
}
