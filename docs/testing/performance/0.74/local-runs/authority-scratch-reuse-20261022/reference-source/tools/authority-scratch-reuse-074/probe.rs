//! Pure synthetic receivers. File is a token, not std::fs::File or an OS FD.
#![cfg(test)]
#[path = "../proc-document-baseline-074/counter.rs"]
mod counter;
use std::cell::{Cell, RefCell};
use std::io::{self, Read};

#[derive(Debug)]
enum AuthorityError {
    Invalid,
    Budget,
    Io(io::Error),
}
impl From<io::Error> for AuthorityError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
#[derive(Clone, Copy)]
struct Metadata {
    dev: u64,
    ino: u64,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl Metadata {
    fn new(length: usize) -> Self {
        Self {
            dev: 7,
            ino: 17,
            uid: 1001,
            gid: 1002,
            mode: 0o100400,
            links: 1,
            len: length as u64,
            mtime: 19,
            mtime_ns: 23,
            ctime: 29,
            ctime_ns: 31,
        }
    }
    fn is_file(&self) -> bool {
        event(Event::Regular);
        self.mode & 0o170000 == 0o100000
    }
    fn dev(&self) -> u64 {
        self.dev
    }
    fn ino(&self) -> u64 {
        self.ino
    }
    fn uid(&self) -> u32 {
        self.uid
    }
    fn gid(&self) -> u32 {
        self.gid
    }
    fn mode(&self) -> u32 {
        self.mode
    }
    fn nlink(&self) -> u64 {
        self.links
    }
    fn len(&self) -> u64 {
        self.len
    }
    fn mtime(&self) -> i64 {
        self.mtime
    }
    fn mtime_nsec(&self) -> i64 {
        self.mtime_ns
    }
    fn ctime(&self) -> i64 {
        self.ctime
    }
    fn ctime_nsec(&self) -> i64 {
        self.ctime_ns
    }
}
#[derive(Clone, Copy)]
struct Scenario {
    before: Metadata,
    after: Metadata,
    held: Metadata,
    parent_dev: u64,
    body_len: usize,
    chunk: usize,
    interrupt_at: usize,
    error_at: usize,
    faults: u8,
    seed: u64,
    flip: usize,
}
const SEED: u64 = 0x074a_17b3;
impl Scenario {
    fn new(length: usize, chunk: usize) -> Self {
        Self {
            before: Metadata::new(length),
            after: Metadata::new(length),
            held: Metadata::new(length),
            parent_dev: 7,
            body_len: length,
            chunk,
            interrupt_at: 0,
            error_at: 0,
            faults: 0,
            seed: SEED,
            flip: usize::MAX,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Empty,
    Held,
    Open,
    Before,
    Regular,
    Parent,
    Read,
    After,
    DropFresh,
    DropHeld,
}
impl Event {
    fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Held => "held",
            Self::Open => "open",
            Self::Before => "before",
            Self::Regular => "regular",
            Self::Parent => "parent",
            Self::Read => "read",
            Self::After => "after",
            Self::DropFresh => "drop-fresh",
            Self::DropHeld => "drop-held",
        }
    }
}
struct State {
    scenario: Scenario,
    order: [Event; 12],
    used: usize,
    reads: usize,
    bytes: usize,
    eof: usize,
    interrupted: usize,
    read_errors: usize,
    max_request: usize,
    fresh_live: usize,
    held_id: Option<u64>,
    next_id: u64,
    first_error: Option<Event>,
}
impl State {
    fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            order: [Event::Empty; 12],
            used: 0,
            reads: 0,
            bytes: 0,
            eof: 0,
            interrupted: 0,
            read_errors: 0,
            max_request: 0,
            fresh_live: 0,
            held_id: None,
            next_id: 1,
            first_error: None,
        }
    }
    fn event(&mut self, value: Event) {
        assert!(self.used < self.order.len());
        self.order[self.used] = value;
        self.used += 1;
    }
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::new(Scenario::new(0, 1))); }
fn event(value: Event) {
    STATE.with(|state| state.borrow_mut().event(value));
}
fn reset(scenario: Scenario) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        assert_eq!(state.fresh_live, 0);
        assert!(state.held_id.is_none());
        *state = State::new(scenario);
    });
}
fn rewind(scenario: Scenario) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        assert_eq!(state.fresh_live, 0);
        let held = state.held_id;
        assert!(held.is_some());
        let next = state.next_id;
        *state = State::new(scenario);
        state.held_id = held;
        state.next_id = next;
    });
}
fn mark_held(file: &File) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        assert_eq!(state.fresh_live, 1);
        assert!(state.held_id.is_none());
        state.fresh_live = 0;
        state.held_id = Some(file.id);
    });
}
fn byte(index: usize, seed: u64) -> u8 {
    match index {
        0 => 0,
        1 => 255,
        2 => b'\n',
        _ => seed
            .rotate_left((index % 64) as u32)
            .wrapping_add(index as u64) as u8,
    }
}
fn fail(state: &mut State, mask: u8, site: Event) -> io::Result<()> {
    if state.scenario.faults & mask != 0 {
        state.first_error = Some(site);
        Err(io::ErrorKind::Other.into())
    } else {
        Ok(())
    }
}
struct File {
    id: u64,
    position: usize,
    metadata_calls: Cell<u8>,
}
impl File {
    fn directory() -> Self {
        Self {
            id: 0,
            position: 0,
            metadata_calls: Cell::new(0),
        }
    }
    fn metadata(&self) -> io::Result<Metadata> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if self.id == 0 {
                state.event(Event::Parent);
                fail(&mut state, 4, Event::Parent)?;
                let mut m = Metadata::new(0);
                m.dev = state.scenario.parent_dev;
                return Ok(m);
            }
            if state.held_id == Some(self.id) {
                state.event(Event::Held);
                fail(&mut state, 32, Event::Held)?;
                return Ok(state.scenario.held);
            }
            let call = self.metadata_calls.get();
            self.metadata_calls.set(call + 1);
            match call {
                0 => {
                    state.event(Event::Before);
                    fail(&mut state, 2, Event::Before)?;
                    Ok(state.scenario.before)
                }
                1 => {
                    state.event(Event::After);
                    fail(&mut state, 16, Event::After)?;
                    Ok(state.scenario.after)
                }
                _ => panic!("unexpected synthetic metadata call"),
            }
        })
    }
}
fn open_at(parent: &File, name: &str, directory: bool) -> Result<File, AuthorityError> {
    assert_eq!(parent.id, 0);
    assert_eq!(name, "record");
    assert!(!directory);
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.event(Event::Open);
        fail(&mut state, 1, Event::Open)?;
        let id = state.next_id;
        state.next_id += 1;
        state.fresh_live += 1;
        assert_eq!(state.fresh_live, 1);
        Ok(File {
            id,
            position: 0,
            metadata_calls: Cell::new(0),
        })
    })
}
impl Read for File {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            assert!(self.id != 0 && state.held_id != Some(self.id));
            assert_eq!(state.fresh_live, 1);
            state.reads += 1;
            if state.reads == 1 {
                state.event(Event::Read);
            }
            state.max_request = state.max_request.max(buffer.len());
            if state.scenario.faults & 8 != 0 || state.scenario.error_at == state.reads {
                state.first_error = Some(Event::Read);
                state.read_errors += 1;
                return Err(io::ErrorKind::Other.into());
            }
            if state.scenario.interrupt_at == state.reads {
                state.interrupted += 1;
                return Err(io::ErrorKind::Interrupted.into());
            }
            let n = buffer
                .len()
                .min(state.scenario.chunk)
                .min(state.scenario.body_len - self.position);
            for (offset, dest) in buffer[..n].iter_mut().enumerate() {
                let index = self.position + offset;
                *dest = byte(index, state.scenario.seed) ^ u8::from(index == state.scenario.flip);
            }
            self.position += n;
            state.bytes += n;
            if n == 0 && !buffer.is_empty() && self.position == state.scenario.body_len {
                state.eof += 1;
            }
            Ok(n)
        })
    }
}
impl Drop for File {
    fn drop(&mut self) {
        if self.id == 0 {
            return;
        }
        STATE.with(|state| {
            let mut state = state.borrow_mut();
            if state.held_id == Some(self.id) {
                state.held_id = None;
                state.event(Event::DropHeld);
            } else {
                assert_eq!(state.fresh_live, 1);
                state.fresh_live = 0;
                state.event(Event::DropFresh);
            }
        });
    }
}
mod model;
