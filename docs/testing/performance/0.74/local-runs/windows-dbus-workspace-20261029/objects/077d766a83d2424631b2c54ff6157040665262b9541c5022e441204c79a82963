//! Pure collection and full document differential tests, never a live owner.
use super::*;
use crate::diagnostic_kernel_admission::tests::projection_prefix::reference;
use crate::diagnostic_kernel_admission::{projection, refusal_trace};

#[test]
fn projection_groups_seeded_values_and_owned_vec_match_reference() {
    let mut seed = 0x0746_7201_u64;
    for length in 0..=33 {
        for _ in 0..64 {
            let input: Vec<_> = (0..length)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    seed as u32
                })
                .collect();
            let output = input.iter().copied().collect::<Values>().into_vec();
            assert_eq!(output, input);
            assert_eq!(output.capacity(), length);
            let mut later = input.iter().copied().collect::<Values>().into_vec();
            later.push(u32::MAX);
            assert_eq!(output, input);
        }
    }
}

#[test]
fn projection_groups_result_short_circuit_keeps_first_error_and_visit_order() {
    for length in 0..=33 {
        for fail in 0..=length {
            let run = |candidate| {
                let mut visited = Vec::new();
                let iter = (0..length).map(|n| {
                    visited.push(n);
                    if n >= fail {
                        Err(n)
                    } else {
                        Ok(n as u32)
                    }
                });
                let result = if candidate {
                    iter.collect::<Result<Values, _>>().map(Values::into_vec)
                } else {
                    iter.collect::<Result<Vec<_>, _>>()
                };
                (result, visited)
            };
            assert_eq!(run(true), run(false));
        }
    }
}

#[test]
fn projection_groups_documents_keep_first_refusal_and_projection_trace() {
    use crate::diagnostic_ipc::validation_trace::kernel_walk;
    let credentials = libc::ucred {
        pid: 23,
        uid: 24,
        gid: 25,
    };
    let compare = |raw: &[u8]| {
        let ((old, old_event), old_walk) =
            kernel_walk::capture(|| refusal_trace::capture(|| reference(raw, &credentials)));
        let ((new, new_event), new_walk) =
            kernel_walk::capture(|| refusal_trace::capture(|| projection(raw, &credentials)));
        assert_eq!(
            old.map_err(|e| e.to_string()),
            new.map_err(|e| e.to_string())
        );
        assert_eq!(
            old_event.map(|e| (e.site, e.clock)),
            new_event.map(|e| (e.site, e.clock))
        );
        assert_eq!(old_walk.roots, new_walk.roots);
        for (old, new) in old_walk.nodes.iter().zip(&new_walk.nodes) {
            assert_eq!(old.calls, new.calls);
        }
        for (old, new) in old_walk.edges.iter().zip(&new_walk.edges) {
            for (old, new) in old.iter().zip(new) {
                assert_eq!(old.calls, new.calls);
            }
        }
    };
    for length in 0..=34 {
        for bad in [None, Some(0), Some(31), Some(32), Some(33)] {
            let groups = (0..length)
                .map(|i| {
                    if Some(i) == bad {
                        "4294967296".into()
                    } else {
                        i.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            let raw = format!(
                "Pid: 23\nTgid: 23\nUid: 24 24 24 24\nGid: 25 25 25 25\nGroups: {groups}\n"
            );
            compare(raw.as_bytes());
            compare(format!("{raw}Groups: bad\n").as_bytes());
            compare(raw.replace("Pid: 23", "Pid: bad").as_bytes());
        }
    }
    for groups in [
        "",
        "2 1",
        "1 1",
        "+1 +2",
        "0 4294967295",
        "1\u{00a0}2",
        "-1",
        "bad",
    ] {
        compare(
            format!("Pid: 23\nTgid: 23\nUid: 24 24 24 24\nGid: 25 25 25 25\nGroups: {groups}\n")
                .as_bytes(),
        );
    }
}
