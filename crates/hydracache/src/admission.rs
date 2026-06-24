use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

/// Admission limits for count, memory and backlog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionLimits {
    /// Maximum concurrent admitted operations.
    pub max_in_flight: usize,
    /// Maximum admitted bytes.
    pub max_memory_bytes: usize,
    /// Maximum queued operations.
    pub max_queue_depth: usize,
    /// Base retry-after used for structured backpressure responses.
    pub base_retry_after_ms: u64,
}

impl AdmissionLimits {
    /// Create limits with fail-loud normalization for zero values.
    pub fn new(max_in_flight: usize, max_memory_bytes: usize, max_queue_depth: usize) -> Self {
        Self {
            max_in_flight: max_in_flight.max(1),
            max_memory_bytes: max_memory_bytes.max(1),
            max_queue_depth,
            base_retry_after_ms: 25,
        }
    }

    /// Set base retry-after.
    pub fn retry_after_ms(mut self, base_retry_after_ms: u64) -> Self {
        self.base_retry_after_ms = base_retry_after_ms.max(1);
        self
    }
}

/// Admission controller with FIFO backlog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionController {
    limits: AdmissionLimits,
    queue: VecDeque<AdmissionQueueTicket>,
    in_flight: usize,
    memory_bytes: usize,
    rejected_total: u64,
}

impl AdmissionController {
    /// Create a controller.
    pub fn new(limits: AdmissionLimits) -> Self {
        Self {
            limits,
            queue: VecDeque::new(),
            in_flight: 0,
            memory_bytes: 0,
            rejected_total: 0,
        }
    }

    /// Try to admit immediately without queueing.
    pub fn try_acquire(
        &mut self,
        request_id: impl Into<String>,
        estimated_bytes: usize,
    ) -> Result<AdmissionPermit, AdmissionError> {
        self.ensure_fits(estimated_bytes)?;
        if self.in_flight >= self.limits.max_in_flight {
            return self.reject(AdmissionRejectionReason::InFlightLimit);
        }
        if self.memory_bytes.saturating_add(estimated_bytes) > self.limits.max_memory_bytes {
            return self.reject(AdmissionRejectionReason::MemoryLimit);
        }
        self.in_flight += 1;
        self.memory_bytes = self.memory_bytes.saturating_add(estimated_bytes);
        Ok(AdmissionPermit {
            request_id: request_id.into(),
            estimated_bytes,
        })
    }

    /// Enqueue an operation for FIFO admission.
    pub fn enqueue(
        &mut self,
        request_id: impl Into<String>,
        estimated_bytes: usize,
    ) -> Result<AdmissionQueueTicket, AdmissionError> {
        self.ensure_fits(estimated_bytes)?;
        if self.queue.len() >= self.limits.max_queue_depth {
            return self.reject(AdmissionRejectionReason::QueueFull);
        }
        let ticket = AdmissionQueueTicket {
            request_id: request_id.into(),
            estimated_bytes,
        };
        self.queue.push_back(ticket.clone());
        Ok(ticket)
    }

    /// Admit the oldest queued operation if capacity is available.
    pub fn admit_next(&mut self) -> Option<AdmissionPermit> {
        let ticket = self.queue.front()?;
        if self.in_flight >= self.limits.max_in_flight
            || self.memory_bytes.saturating_add(ticket.estimated_bytes)
                > self.limits.max_memory_bytes
        {
            return None;
        }
        let ticket = self.queue.pop_front().expect("front was present");
        self.in_flight += 1;
        self.memory_bytes = self.memory_bytes.saturating_add(ticket.estimated_bytes);
        Some(AdmissionPermit {
            request_id: ticket.request_id,
            estimated_bytes: ticket.estimated_bytes,
        })
    }

    /// Release a completed operation.
    pub fn release(&mut self, permit: AdmissionPermit) {
        self.in_flight = self.in_flight.saturating_sub(1);
        self.memory_bytes = self.memory_bytes.saturating_sub(permit.estimated_bytes);
    }

    /// Return current controller snapshot.
    pub fn snapshot(&self) -> AdmissionSnapshot {
        AdmissionSnapshot {
            in_flight: self.in_flight,
            memory_bytes: self.memory_bytes,
            queue_depth: self.queue.len(),
            rejected_total: self.rejected_total,
        }
    }

    fn ensure_fits(&mut self, estimated_bytes: usize) -> Result<(), AdmissionError> {
        if estimated_bytes > self.limits.max_memory_bytes {
            return self.reject(AdmissionRejectionReason::MemoryLimit);
        }
        Ok(())
    }

    fn reject<T>(&mut self, reason: AdmissionRejectionReason) -> Result<T, AdmissionError> {
        self.rejected_total = self.rejected_total.saturating_add(1);
        Err(AdmissionError::Backpressure {
            reason,
            retry_after_ms: self.retry_after_ms(),
        })
    }

    fn retry_after_ms(&self) -> u64 {
        self.limits
            .base_retry_after_ms
            .saturating_mul((self.queue.len() as u64).saturating_add(1))
    }
}

/// Permit returned for admitted work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionPermit {
    /// Request id supplied by the caller.
    pub request_id: String,
    /// Estimated bytes charged to the controller.
    pub estimated_bytes: usize,
}

/// FIFO queue ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionQueueTicket {
    /// Request id supplied by the caller.
    pub request_id: String,
    /// Estimated bytes requested.
    pub estimated_bytes: usize,
}

/// Admission controller snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionSnapshot {
    /// Current admitted operation count.
    pub in_flight: usize,
    /// Current admitted bytes.
    pub memory_bytes: usize,
    /// Waiting FIFO backlog depth.
    pub queue_depth: usize,
    /// Total rejected operations.
    pub rejected_total: u64,
}

/// Retryable rejection reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionRejectionReason {
    /// Concurrency limit would be exceeded.
    InFlightLimit,
    /// Memory/bytes limit would be exceeded.
    MemoryLimit,
    /// FIFO backlog is full.
    QueueFull,
}

/// Admission error returned instead of unbounded queueing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    /// Retryable overload signal.
    Backpressure {
        /// Reason for rejection.
        reason: AdmissionRejectionReason,
        /// Suggested retry-after in milliseconds.
        retry_after_ms: u64,
    },
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backpressure {
                reason,
                retry_after_ms,
            } => write!(
                formatter,
                "admission backpressure: {reason:?}; retry after {retry_after_ms} ms"
            ),
        }
    }
}

impl Error for AdmissionError {}
