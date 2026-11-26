//! Owner-side decisions over one certified record and bounded observation.

use hydracache_client_protocol::{
    CasExpectation, ClientRequest, ClientResponse, CompareValueExpireMode, ConditionalPutCondition,
    LockConsistency, Namespace, StructuredKey,
};

use crate::imap_value_journal::{ExpiryCandidate, StoredEntry};

pub(crate) type Bounds = (u64, u64);

#[derive(Clone, Copy)]
pub(crate) enum WriteOp<'a> {
    Put(&'a [u8], Option<u64>),
    Delete,
    PutIf(&'a [u8], Option<u64>, &'a ConditionalPutCondition),
    Cas(&'a CasExpectation, &'a [u8]),
    RemoveIf(&'a [u8], bool),
    Expire(u64),
    Persist,
    Extend(&'a [u8], u64, CompareValueExpireMode),
    ExpireDue(&'a ExpiryCandidate),
}

impl<'a> WriteOp<'a> {
    pub fn request(request: &'a ClientRequest) -> Option<(&'a Namespace, &'a StructuredKey, Self)> {
        Some(match request {
            ClientRequest::Put {
                ns,
                key,
                value,
                ttl_ms,
                dimensions,
            } if dimensions.is_empty() => (ns, key, Self::Put(value, *ttl_ms)),
            ClientRequest::Invalidate { ns, key } => (ns, key, Self::Delete),
            ClientRequest::ConditionalPut {
                ns,
                key,
                value,
                ttl_ms,
                condition,
            } => (ns, key, Self::PutIf(value, *ttl_ms, condition)),
            ClientRequest::CompareAndSet {
                ns,
                key,
                expected,
                new_value,
                level,
            } if *level != LockConsistency::One => (ns, key, Self::Cas(expected, new_value)),
            ClientRequest::RemoveIfValue {
                ns,
                key,
                expected,
                level,
            } if *level != LockConsistency::One => (ns, key, Self::RemoveIf(expected, true)),
            ClientRequest::CompareValueAndInvalidate {
                ns,
                key,
                expected_value,
            } => (ns, key, Self::RemoveIf(expected_value, false)),
            ClientRequest::Expire { ns, key, ttl_ms } => (ns, key, Self::Expire(*ttl_ms)),
            ClientRequest::Persist { ns, key } => (ns, key, Self::Persist),
            ClientRequest::CompareValueAndExpire {
                ns,
                key,
                expected_value,
                ttl_ms,
                mode,
            } => (ns, key, Self::Extend(expected_value, *ttl_ms, *mode)),
            _ => return None,
        })
    }

    pub fn fits(self, limit: usize) -> bool {
        match self {
            Self::Put(value, _) => value.len() <= limit,
            Self::PutIf(value, _, condition) => {
                value.len() <= limit
                    && match condition {
                        ConditionalPutCondition::IfAbsent => true,
                        ConditionalPutCondition::IfPresentValue(expected) => {
                            expected.len() <= limit
                        }
                    }
            }
            Self::Cas(expected, value) => {
                value.len() <= limit
                    && match expected {
                        CasExpectation::Present => true,
                        CasExpectation::Exact(expected) => expected.len() <= limit,
                    }
            }
            Self::RemoveIf(expected, _) | Self::Extend(expected, _, _) => expected.len() <= limit,
            _ => true,
        }
    }

    pub fn needs_current(self) -> bool {
        !matches!(self, Self::Put(..) | Self::Delete)
    }

    pub fn decide(
        self,
        current: Option<&StoredEntry>,
        time: Option<Bounds>,
        version: u64,
    ) -> Option<Decision> {
        if let Self::ExpireDue(ticket) = self {
            let due = current.is_some_and(|entry| {
                entry.record.version == ticket.version
                    && entry.expires_not_before_ms == Some(ticket.deadline)
            }) && time.is_some_and(|(lower, _)| lower >= ticket.deadline);
            return Some(Decision {
                change: due.then_some(Change::tombstone()),
                response: ClientResponse::Expiry { applied: due },
            });
        }
        let current = match current {
            Some(entry) if entry.value().is_some() => match entry.expires_not_before_ms {
                Some(deadline) => {
                    let (lower, upper) = time?;
                    if lower >= deadline {
                        None
                    } else if upper >= deadline {
                        return None;
                    } else {
                        Some(entry)
                    }
                }
                None => Some(entry),
            },
            _ => None,
        };
        let live = current.and_then(StoredEntry::value);
        let expiry = current.and_then(|entry| entry.expires_not_before_ms);
        let (change, response) = match self {
            Self::Put(value, ttl) => (
                Some(Change::with_ttl(value, ttl, time)?),
                ClientResponse::Stored,
            ),
            Self::Delete => (Some(Change::tombstone()), ClientResponse::Invalidated),
            Self::PutIf(value, ttl, condition) => {
                let matches = match condition {
                    ConditionalPutCondition::IfAbsent => live.is_none(),
                    ConditionalPutCondition::IfPresentValue(expected) => live == Some(expected),
                };
                let change = if matches {
                    Some(Change::with_ttl(value, ttl, time)?)
                } else {
                    None
                };
                (
                    change,
                    ClientResponse::ConditionalStored { stored: matches },
                )
            }
            Self::Cas(expected, value) => {
                let matches = match expected {
                    CasExpectation::Exact(expected) => live == Some(expected),
                    CasExpectation::Present => live.is_some(),
                };
                if matches {
                    (
                        Some(Change::value(value, None)),
                        ClientResponse::CasApplied {
                            new_version: version,
                        },
                    )
                } else {
                    (
                        None,
                        ClientResponse::CasMismatch {
                            current: live.map(<[u8]>::to_vec),
                        },
                    )
                }
            }
            Self::RemoveIf(expected, cas) => {
                let matches = live == Some(expected);
                let response = match (cas, matches) {
                    (true, true) => ClientResponse::CasApplied {
                        new_version: version,
                    },
                    (true, false) => ClientResponse::CasMismatch {
                        current: live.map(<[u8]>::to_vec),
                    },
                    (false, applied) => ClientResponse::CompareValueApplied { applied },
                };
                (matches.then_some(Change::tombstone()), response)
            }
            Self::Expire(ttl) => {
                let change = match live {
                    Some(value) => Some(Change::with_ttl(value, Some(ttl), time)?),
                    None => None,
                };
                let applied = change.is_some();
                (change, ClientResponse::Expiry { applied })
            }
            Self::Persist => {
                let change = live
                    .filter(|_| expiry.is_some())
                    .map(|value| Change::value(value, None));
                let applied = change.is_some();
                (change, ClientResponse::Expiry { applied })
            }
            Self::Extend(expected, ttl, mode) => {
                let change = if live != Some(expected) {
                    None
                } else {
                    match (mode, expiry) {
                        (CompareValueExpireMode::Replace, _)
                        | (CompareValueExpireMode::ReplaceIfExpiring, Some(_)) => {
                            Some(Change::with_ttl(expected, Some(ttl), time)?)
                        }
                        (CompareValueExpireMode::AddToRemaining, Some(expiry)) => {
                            Some(Change::value(expected, Some(expiry.checked_add(ttl)?)))
                        }
                        _ => None,
                    }
                };
                let applied = change.is_some();
                (change, ClientResponse::CompareValueApplied { applied })
            }
            Self::ExpireDue(_) => unreachable!("checked before ordinary mutations"),
        };
        Some(Decision { change, response })
    }
}

pub(crate) struct Decision {
    pub change: Option<Change>,
    pub response: ClientResponse,
}

pub(crate) struct Change {
    pub value: Option<Vec<u8>>,
    pub expiry: Option<u64>,
}

impl Change {
    pub fn tombstone() -> Self {
        Self {
            value: None,
            expiry: None,
        }
    }

    fn value(value: &[u8], expiry: Option<u64>) -> Self {
        Self {
            value: Some(value.to_vec()),
            expiry,
        }
    }

    fn with_ttl(value: &[u8], ttl: Option<u64>, time: Option<Bounds>) -> Option<Self> {
        match ttl {
            None => Some(Self::value(value, None)),
            Some(0) => Some(Self::tombstone()),
            Some(ttl) => Some(Self::value(value, Some(time?.1.checked_add(ttl)?))),
        }
    }
}
