//! Synthetic brackets/routing/history, not the filesystem publication engine.
use super::*;
use crate::{event, finish_leaf, mark_held, reset, select, Event, Scenario, SEED, STATE};
use std::marker::PhantomData;
use std::rc::Rc;

pub(super) const GENERATIONS: [usize; 5] = [1, 2, 16, 64, 128];
pub(super) trait Storage: Sized {
    fn new(original: Leaf) -> Self;
    fn original(&self) -> &Leaf;
    fn revalidate(&mut self, dir: &File, budget: usize) -> Result<(), AuthorityError>;
    fn evict(&mut self);
    fn capacity(&self) -> usize;
}

#[path = "operator.rs"]
pub(super) mod operator;

#[path = "../authority-owned-journal-074/lifecycle.rs"]
mod lifecycle;
impl Storage for Leaf {
    fn new(original: Leaf) -> Self {
        original
    }
    fn original(&self) -> &Leaf {
        self
    }
    fn revalidate(&mut self, dir: &File, budget: usize) -> Result<(), AuthorityError> {
        Leaf::revalidate(self, dir, 1001, 1002, budget)
    }
    fn evict(&mut self) {}
    fn capacity(&self) -> usize {
        0
    }
}
pub(super) fn normal(slot: usize) -> Scenario {
    let length = if slot.is_multiple_of(2) { 2048 } else { 65 };
    let mut s = Scenario::new(length, length);
    s.seed = SEED ^ (slot as u64).wrapping_mul(0x9e3779b97f4a7c15);
    for m in [&mut s.before, &mut s.after, &mut s.held] {
        m.ino += slot as u64;
    }
    s
}
#[derive(Clone, Copy, Default)]
pub(super) struct Fault {
    pub gate: Option<Event>,
    pub leaf: Option<(usize, Scenario)>,
}
fn gate(site: Event, fault: Fault) -> Result<(), AuthorityError> {
    STATE.with(|state| state.borrow_mut().slot = 256);
    event(site);
    if fault.gate == Some(site) {
        Err(AuthorityError::Invalid)
    } else {
        Ok(())
    }
}
pub(super) struct Entry<L> {
    pub record: L,
    pub marker: L,
}
pub(super) struct Journal<L> {
    pub entries: Vec<Entry<L>>,
    pub failed: bool,
    pub refused: bool,
    pub epoch: u64,
    _thread: PhantomData<Rc<()>>,
}
impl<L: Storage> Journal<L> {
    pub fn build(generations: usize) -> Self {
        assert!(generations <= 128);
        reset(Scenario::new(0, 1));
        let mut read = Self {
            entries: Vec::with_capacity(1),
            failed: false,
            refused: false,
            epoch: 1,
            _thread: PhantomData,
        };
        for _ in 0..generations {
            read.add_entry();
        }
        read
    }
    fn add_entry(&mut self) {
        let dir = File::directory();
        let open = |slot| {
            let s = normal(slot);
            select(slot, s);
            let original = Leaf::open(&dir, "record".into(), 1001, 1002, s.body_len).unwrap();
            mark_held(&original.file);
            L::new(original)
        };
        let slot = self.entries.len() * 2;
        let record = open(slot);
        let marker = open(slot + 1);
        self.entries.push(Entry { record, marker });
    }
    pub fn evict(&mut self) {
        for entry in &mut self.entries {
            entry.record.evict();
            entry.marker.evict();
        }
    }
    pub fn latch(&mut self) {
        self.failed = true;
        self.evict();
    }
    pub fn capacity(&self) -> usize {
        self.entries
            .iter()
            .map(|e| e.record.capacity() + e.marker.capacity())
            .sum()
    }
    pub fn originals(&self) -> Vec<(u64, u64)> {
        self.entries
            .iter()
            .flat_map(|e| [&e.record, &e.marker])
            .map(|leaf| {
                let leaf = leaf.original();
                let hash = leaf.raw.iter().fold(0xcbf29ce484222325, |h, b| {
                    (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
                });
                (leaf.file.id, hash)
            })
            .collect()
    }
    pub fn revalidate(&mut self, fault: Fault) -> Result<(), AuthorityError> {
        if self.failed {
            return Err(AuthorityError::Refused);
        }
        let result = (|| {
            gate(Event::DirBefore, fault)?;
            gate(Event::InvBefore, fault)?;
            let dir = File::directory();
            for (index, entry) in self.entries.iter_mut().enumerate() {
                for (slot, leaf) in [
                    (index * 2, &mut entry.record),
                    (index * 2 + 1, &mut entry.marker),
                ] {
                    let s = if let Some((target, s)) = fault.leaf {
                        if target == slot {
                            s
                        } else {
                            normal(slot)
                        }
                    } else {
                        normal(slot)
                    };
                    select(slot, s);
                    let result = leaf.revalidate(&dir, normal(slot).body_len);
                    finish_leaf();
                    result?;
                }
            }
            gate(Event::DirAfter, fault)?;
            gate(Event::InvAfter, fault)?;
            Ok(())
        })();
        if result.is_err() {
            self.latch();
        }
        result
    }
}
