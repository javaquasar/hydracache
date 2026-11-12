//! Exact ownership ledger for proving timeout, cancellation and crash cleanup.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    Dedup,
    Listener,
    Staging,
    Quota,
    Bulk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    InvalidBound,
    InvalidReservation,
    Capacity,
    UnknownReservation,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ResourceError {}

#[derive(Debug, Clone, Copy)]
struct Reservation {
    request: u64,
    kind: ResourceKind,
    units: usize,
}

#[derive(Debug)]
pub struct ResourceLedger {
    max_units: usize,
    next_id: u64,
    used_units: usize,
    reservations: BTreeMap<u64, Reservation>,
}

impl ResourceLedger {
    pub fn new(max_units: usize) -> Result<Self, ResourceError> {
        if max_units == 0 {
            return Err(ResourceError::InvalidBound);
        }
        Ok(Self {
            max_units,
            next_id: 1,
            used_units: 0,
            reservations: BTreeMap::new(),
        })
    }

    pub fn reserve(
        &mut self,
        request: u64,
        kind: ResourceKind,
        units: usize,
    ) -> Result<u64, ResourceError> {
        if request == 0 || units == 0 {
            return Err(ResourceError::InvalidReservation);
        }
        if self.used_units.saturating_add(units) > self.max_units {
            return Err(ResourceError::Capacity);
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or(ResourceError::Capacity)?;
        self.reservations.insert(
            id,
            Reservation {
                request,
                kind,
                units,
            },
        );
        self.used_units += units;
        Ok(id)
    }

    pub fn release(&mut self, id: u64) -> Result<(), ResourceError> {
        let reservation = self
            .reservations
            .remove(&id)
            .ok_or(ResourceError::UnknownReservation)?;
        self.used_units -= reservation.units;
        Ok(())
    }

    pub fn cleanup_request(&mut self, request: u64) -> usize {
        let ids = self
            .reservations
            .iter()
            .filter_map(|(id, reservation)| (reservation.request == request).then_some(*id))
            .collect::<Vec<_>>();
        let mut released = 0;
        for id in ids {
            let reservation = self
                .reservations
                .remove(&id)
                .expect("reservation was collected from the same ledger");
            self.used_units -= reservation.units;
            released += reservation.units;
        }
        released
    }

    pub fn units_by_kind(&self, kind: ResourceKind) -> usize {
        self.reservations
            .values()
            .filter(|reservation| reservation.kind == kind)
            .map(|reservation| reservation.units)
            .sum()
    }

    pub const fn used_units(&self) -> usize {
        self.used_units
    }

    pub fn is_clean(&self) -> bool {
        self.used_units == 0 && self.reservations.is_empty()
    }
}
