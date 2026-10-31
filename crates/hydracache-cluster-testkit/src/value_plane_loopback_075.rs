//! Test-only logical transport. This is not a production wire format.

use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopbackFault {
    Deliver,
    Drop,
    Duplicate,
    Delay(u64),
    Corrupt,
    Partial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackFrame {
    pub generation: u64,
    pub sequence: u64,
    pub payload: Vec<u8>,
    checksum: [u8; 32],
}

impl LoopbackFrame {
    pub fn new(generation: u64, sequence: u64, payload: Vec<u8>) -> Self {
        let checksum = digest(generation, sequence, &payload);
        Self {
            generation,
            sequence,
            payload,
            checksum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopbackError {
    InvalidBound,
    InvalidFrame,
    PayloadTooLarge,
    StaleGeneration,
    CorruptFrame,
}

impl fmt::Display for LoopbackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LoopbackError {}

#[derive(Debug, Clone)]
struct QueuedFrame {
    deliver_at: u64,
    frame: LoopbackFrame,
}

#[derive(Debug)]
pub struct LoopbackTransport {
    max_payload: usize,
    generation: u64,
    tick: u64,
    queue: VecDeque<QueuedFrame>,
    trace: Vec<&'static str>,
}

impl LoopbackTransport {
    pub fn new(max_payload: usize, generation: u64) -> Result<Self, LoopbackError> {
        if max_payload == 0 {
            return Err(LoopbackError::InvalidBound);
        }
        if generation == 0 {
            return Err(LoopbackError::InvalidFrame);
        }
        Ok(Self {
            max_payload,
            generation,
            tick: 0,
            queue: VecDeque::new(),
            trace: Vec::new(),
        })
    }

    pub fn send(
        &mut self,
        mut frame: LoopbackFrame,
        fault: LoopbackFault,
    ) -> Result<(), LoopbackError> {
        if frame.generation < self.generation {
            return Err(LoopbackError::StaleGeneration);
        }
        if frame.generation == 0 || frame.sequence == 0 {
            return Err(LoopbackError::InvalidFrame);
        }
        if frame.payload.len() > self.max_payload {
            return Err(LoopbackError::PayloadTooLarge);
        }
        if frame.generation > self.generation {
            self.generation = frame.generation;
        }
        match fault {
            LoopbackFault::Drop => self.trace.push("dropped"),
            LoopbackFault::Deliver => self.enqueue(frame, self.tick, "queued"),
            LoopbackFault::Duplicate => {
                self.enqueue(frame.clone(), self.tick, "queued");
                self.enqueue(frame, self.tick, "duplicated");
            }
            LoopbackFault::Delay(ticks) => {
                self.enqueue(frame, self.tick.saturating_add(ticks.max(1)), "delayed");
            }
            LoopbackFault::Corrupt => {
                frame.checksum[0] ^= 0xff;
                self.enqueue(frame, self.tick, "corrupted");
            }
            LoopbackFault::Partial => {
                frame.payload.pop();
                self.enqueue(frame, self.tick, "partial");
            }
        }
        Ok(())
    }

    pub fn advance(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    pub fn receive(&mut self) -> Result<Option<LoopbackFrame>, LoopbackError> {
        let Some(position) = self
            .queue
            .iter()
            .position(|queued| queued.deliver_at <= self.tick)
        else {
            return Ok(None);
        };
        let queued = self
            .queue
            .remove(position)
            .expect("position came from queue");
        if digest(
            queued.frame.generation,
            queued.frame.sequence,
            &queued.frame.payload,
        ) != queued.frame.checksum
        {
            return Err(LoopbackError::CorruptFrame);
        }
        self.trace.push("delivered");
        Ok(Some(queued.frame))
    }

    pub fn trace(&self) -> &[&'static str] {
        &self.trace
    }

    fn enqueue(&mut self, frame: LoopbackFrame, deliver_at: u64, action: &'static str) {
        self.queue.push_back(QueuedFrame { deliver_at, frame });
        self.trace.push(action);
    }
}

fn digest(generation: u64, sequence: u64, payload: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(generation.to_be_bytes());
    digest.update(sequence.to_be_bytes());
    digest.update((payload.len() as u64).to_be_bytes());
    digest.update(payload);
    digest.finalize().into()
}
