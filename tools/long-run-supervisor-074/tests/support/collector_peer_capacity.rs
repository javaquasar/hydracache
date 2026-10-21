//! Pure actual peer capacity candidate and frozen reference; no live observations.
use super::*;
use crate::diagnostic_ipc::DiagnosticPeer;
use crate::diagnostic_kernel_admission::collector_inputs::capacity_controls::candidate_peer;
use crate::diagnostic_kernel_admission::KernelError;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

fn peer(uid: u32, gid: u32, supplementary: &[u32]) -> DiagnosticPeer {
    let mut gids = supplementary.to_vec();
    gids.push(gid);
    gids.sort_unstable();
    gids.dedup();
    DiagnosticPeer { uid, gids }
}

fn cases() -> Vec<(usize, String, u32, Vec<u32>)> {
    let mut cases = vec![(0, "empty".into(), 1, vec![])];
    for length in [1, 2, 3, 4, 8, 16, 31, 32, 33, 64] {
        let distinct = (0..length).map(|i| 2 * i as u32 + 2).collect::<Vec<_>>();
        cases.push((length, "absent".into(), 1, distinct.clone()));
        cases.push((length, "present".into(), 2, distinct));
        cases.push((length, "duplicates".into(), 2, vec![2; length]));
    }
    cases
}
fn oracle(primary: u32, groups: &[u32]) -> Vec<u32> {
    let mut values = groups.iter().copied().collect::<BTreeSet<_>>();
    values.insert(primary);
    values.into_iter().collect()
}

#[test]
fn collector_peer_capacity_complete_operation_requests() {
    for (length, shape, primary, groups) in cases() {
        let expected = oracle(primary, &groups);
        let example = candidate_peer(17, primary, &groups);
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                let value = candidate_peer(17, primary, std::hint::black_box(&groups));
                let observed = std::hint::black_box(&value);
                assert_eq!(observed.uid, 17);
                assert_eq!(observed.gids, expected);
            }
        });
        assert_eq!(counts.allocations, 256);
        assert_eq!(counts.reallocations, 0);
        assert_eq!(counts.requested_bytes, (length as u64 + 1) * 4 * 256);
        assert_eq!(example.gids.capacity(), length + 1);
        println!(
            "HC074-COLLECTOR-PEER-CAPACITY {}",
            serde_json::json!({
                "length":length,"shape":shape,"primary_present":(groups.contains(&primary)),
                "final_length":example.gids.len(),"final_capacity":example.gids.capacity(),
                "iterations":256,"counts":counts,"rust_requests_only":true,
                "timing_claim":false,"profile":false,"server":false,"product":false
            })
        );
    }
}

#[test]
fn collector_peer_capacity_stage_requests_match_one_complete_construction() {
    for (length, shape, primary, groups) in cases() {
        let (mut gids, reserved) = capture(|| Vec::with_capacity(groups.len() + 1));
        let (_, copied) = capture(|| gids.extend_from_slice(&groups));
        let (_, pushed) = capture(|| gids.push(primary));
        let (_, canonical) = capture(|| {
            gids.sort_unstable();
            gids.dedup();
        });
        let (whole, complete) =
            capture(|| candidate_peer(17, primary, std::hint::black_box(&groups)));
        assert_eq!(whole.gids, gids);
        assert_eq!(whole.gids, oracle(primary, &groups));
        assert_eq!(reserved.allocations, 1);
        assert_eq!(reserved.reallocations, 0);
        assert_eq!(reserved.requested_bytes, (length as u64 + 1) * 4);
        assert_eq!(copied, Counts::default());
        assert_eq!(pushed, Counts::default());
        assert_eq!(canonical, Counts::default());
        assert_eq!(complete, reserved);
        assert_eq!(gids.capacity(), length + 1);
        println!(
            "HC074-COLLECTOR-PEER-CAPACITY-STAGES {}",
            serde_json::json!({
                "length":length,"shape":shape,"constructions":1,"reserve":reserved,
                "copy":copied,"push":pushed,"canonical":canonical,"complete":complete,
                "final_capacity":gids.capacity(),"model_only":true,"timing_claim":false
            })
        );
    }
}

#[test]
fn collector_peer_capacity_seeded_canonical_union_and_unmodified_inputs() {
    let mut seed = 0x074c_0ee1_u64;
    for sample in 0_usize..1024 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let length = [0_usize, 1, 2, 3, 4, 8, 16, 31, 32, 33, 64][sample % 11];
        let mut groups = Vec::new();
        for i in 0..length {
            let value = if i.is_multiple_of(13) {
                u32::MAX
            } else {
                (seed.rotate_left(i as u32) % 37) as u32
            };
            groups.push(value);
        }
        let uid = if sample.is_multiple_of(7) {
            u32::MAX
        } else {
            seed as u32
        };
        let primary = if sample.is_multiple_of(5) {
            0
        } else {
            (seed >> 32) as u32
        };
        let before = groups.clone();
        let expected = oracle(primary, &groups);
        let reference = peer(uid, primary, &groups);
        let result = candidate_peer(uid, primary, &groups);
        assert_eq!(result.uid, reference.uid);
        assert_eq!(result.gids, reference.gids);
        assert_eq!(result.uid, uid);
        assert_eq!(result.gids, expected, "seed=0x074c0ee1 sample={sample}");
        assert_eq!(groups, before);
        groups.clear();
        groups.push(12345);
        assert_eq!(result.gids, expected);
    }
    println!("HC074-COLLECTOR-PEER-CAPACITY-UNION seed=0x074c0ee1 samples=1024 model_only=true");
}

#[test]
fn collector_peer_capacity_boundary_union_and_owned_outputs_without_truncation() {
    for length in [0, 1, 31, 32, 33, 64, 256] {
        let input = (2..length as u32 + 2).rev().collect::<Vec<_>>();
        let reference = peer(0, 1, &input);
        let outer = candidate_peer(0, 1, &input);
        assert_eq!(outer.gids, reference.gids);
        assert_eq!(outer.gids.capacity(), length + 1);
        assert_eq!(outer.gids.len(), length + 1);
        assert_eq!(outer.gids, oracle(1, &input));
        let inner = candidate_peer(u32::MAX, 0, &[u32::MAX, 0, u32::MAX]);
        assert_eq!(inner.gids, [0, u32::MAX]);
        assert_eq!(inner.uid, u32::MAX);
        assert_eq!(outer.gids, oracle(1, &input));
    }
    assert_eq!(
        candidate_peer(17, 1, &(2..34).collect::<Vec<_>>())
            .gids
            .len(),
        33
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Sender(u8),
    Sample,
    Hold,
    Peer,
    Seconds,
    Latch,
}
struct Script {
    events: RefCell<Vec<Event>>,
    sender_calls: Cell<u8>,
    faults: u8,
}
struct MockCredentials {
    uid: u32,
    gid: u32,
}
struct MockProjection {
    groups: Vec<u32>,
}
struct MockSender {
    credentials: MockCredentials,
    projection: MockProjection,
    script: Rc<Script>,
}
struct MockOrigin {
    script: Rc<Script>,
}
struct MockPoint {
    script: Rc<Script>,
}
impl MockSender {
    fn revalidate(&self, origin: &MockOrigin) -> Result<(), KernelError> {
        assert!(Rc::ptr_eq(&self.script, &origin.script));
        let call = self.script.sender_calls.get() + 1;
        self.script.sender_calls.set(call);
        self.script.events.borrow_mut().push(Event::Sender(call));
        if call == 1 && self.script.faults & 1 != 0 {
            return Err(KernelError::NotLive);
        }
        if call == 2 && self.script.faults & 8 != 0 {
            return Err(std::io::Error::from_raw_os_error(libc::EBADF).into());
        }
        Ok(())
    }
}
impl MockOrigin {
    fn sample(&self) -> Result<MockPoint, KernelError> {
        self.script.events.borrow_mut().push(Event::Sample);
        if self.script.faults & 2 != 0 {
            return Err(std::io::Error::from_raw_os_error(libc::EIO).into());
        }
        Ok(MockPoint {
            script: Rc::clone(&self.script),
        })
    }
}
impl MockPoint {
    fn unix_seconds(&self) -> u64 {
        self.script.events.borrow_mut().push(Event::Seconds);
        17
    }
}
fn valid_hold(received: &Rc<Script>, now: &MockPoint) -> Result<(), KernelError> {
    assert!(Rc::ptr_eq(received, &now.script));
    received.events.borrow_mut().push(Event::Hold);
    if received.faults & 4 != 0 {
        return Err(KernelError::Invalid);
    }
    Ok(())
}
struct MockPacket {
    candidate: bool,
    refused: bool,
    sender: MockSender,
    origin: MockOrigin,
    received: Rc<Script>,
}
impl MockPacket {
    fn latch_refusal(&mut self) {
        self.refused = true;
        self.received.events.borrow_mut().push(Event::Latch);
    }
    fn project_peer(&self, uid: u32, gid: u32, supplementary: &[u32]) -> DiagnosticPeer {
        self.received.events.borrow_mut().push(Event::Peer);
        if self.candidate {
            candidate_peer(uid, gid, supplementary)
        } else {
            peer(uid, gid, supplementary)
        }
    }
    fn original_collector_inputs(&mut self) -> Result<(DiagnosticPeer, u64), KernelError> {
        let result = (|| {
            if self.refused {
                return Err(KernelError::Refused);
            }
            self.sender.revalidate(&self.origin)?;
            let now = self.origin.sample()?;
            valid_hold(&self.received, &now)?;
            let peer = self.project_peer(
                self.sender.credentials.uid,
                self.sender.credentials.gid,
                &self.sender.projection.groups,
            );
            self.sender.revalidate(&self.origin)?;
            Ok((peer, now.unix_seconds()))
        })();
        if result.is_err() {
            self.latch_refusal();
        }
        result
    }
}
fn outcome(
    value: Result<(DiagnosticPeer, u64), KernelError>,
) -> Result<(u32, Vec<u32>, u64), String> {
    value
        .map(|(peer, time)| (peer.uid, peer.gids, time))
        .map_err(|error| match error {
            KernelError::Refused => "refused".into(),
            KernelError::NotLive => "not-live".into(),
            KernelError::Invalid => "invalid".into(),
            KernelError::Io(e) => format!("io:{:?}", e.raw_os_error()),
            e => panic!("unexpected pure error: {e}"),
        })
}
#[test]
fn collector_peer_capacity_wrapper_first_errors_latch_and_reentry_schedule() {
    for candidate in [false, true] {
        for initially_refused in [false, true] {
            for faults in 0..16 {
                let script = Rc::new(Script {
                    events: RefCell::new(Vec::new()),
                    sender_calls: Cell::new(0),
                    faults,
                });
                let mut packet = MockPacket {
                    candidate,
                    refused: initially_refused,
                    sender: MockSender {
                        credentials: MockCredentials { uid: 23, gid: 4 },
                        projection: MockProjection {
                            groups: vec![8, 4, 8, 2],
                        },
                        script: Rc::clone(&script),
                    },
                    origin: MockOrigin {
                        script: Rc::clone(&script),
                    },
                    received: Rc::clone(&script),
                };
                let mut expected = Vec::new();
                let error = if initially_refused {
                    Some("refused".to_owned())
                } else {
                    expected.push(Event::Sender(1));
                    if faults & 1 != 0 {
                        Some("not-live".into())
                    } else {
                        expected.push(Event::Sample);
                        if faults & 2 != 0 {
                            Some(format!("io:{:?}", Some(libc::EIO)))
                        } else {
                            expected.push(Event::Hold);
                            if faults & 4 != 0 {
                                Some("invalid".into())
                            } else {
                                expected.extend([Event::Peer, Event::Sender(2)]);
                                if faults & 8 != 0 {
                                    Some(format!("io:{:?}", Some(libc::EBADF)))
                                } else {
                                    expected.push(Event::Seconds);
                                    None
                                }
                            }
                        }
                    }
                };
                let expected_result = if let Some(error) = error {
                    expected.push(Event::Latch);
                    Err(error)
                } else {
                    Ok((23, vec![2, 4, 8], 17))
                };
                let result = outcome(packet.original_collector_inputs());
                assert_eq!(result, expected_result);
                assert_eq!(*script.events.borrow(), expected);
                assert_eq!(packet.refused, result.is_err());
                if result.is_err() {
                    script.events.borrow_mut().clear();
                    assert_eq!(
                        outcome(packet.original_collector_inputs()),
                        Err("refused".into())
                    );
                    assert_eq!(*script.events.borrow(), [Event::Latch]);
                }
            }
        }
    }
    println!("HC074-COLLECTOR-PEER-CAPACITY-WRAPPER cases=32 arms=2 model_only=true");
}
