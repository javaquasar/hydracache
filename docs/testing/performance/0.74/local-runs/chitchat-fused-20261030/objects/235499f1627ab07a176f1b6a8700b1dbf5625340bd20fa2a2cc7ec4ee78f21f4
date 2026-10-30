//! Actual fixed-MTU caller; constructor through final wire/drop is one region.
use super::*;
use std::cell::Cell;
thread_local! {static COMPRESSIONS:Cell<u64>=const{Cell::new(0)};}
pub(crate) fn record_compression() {
    COMPRESSIONS.with(|v| v.set(v.get() + 1));
}
#[derive(serde::Deserialize, serde::Serialize)]
struct Table {
    seed: u64,
    cases: Vec<Case>,
}
#[derive(serde::Deserialize, serde::Serialize)]
struct Case {
    name: String,
    mtu: usize,
    requests: Vec<Request>,
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
enum Request {
    #[serde(rename = "node")]
    Node { id: String, port: u16 },
    #[serde(rename = "kv")]
    Kv {
        key: String,
        value: String,
        version: u64,
    },
    #[serde(rename = "max")]
    Max { version: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
struct Inventory {
    len: [usize; 3],
    capacity: [usize; 3],
    cache: Option<usize>,
}
fn inventory(s: &DeltaSerializer) -> Inventory {
    let (len, capacity, cache) = s.compressed_stream_writer.caller_inventory();
    Inventory {
        len,
        capacity,
        cache,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Trace {
    states: [Inventory; 17],
    decisions: [bool; 16],
    attempted: usize,
    wire: [u8; 65536],
    wire_len: usize,
    calls: u64,
    paths: [u64; 4],
}
fn table() -> Table {
    let t: Table = serde_json::from_str(include_str!("cases-r2.json")).unwrap();
    assert_eq!(t.seed, 0x0741028);
    assert_eq!(t.cases.len(), 30);
    t
}
fn run(case: &Case) -> Trace {
    COMPRESSIONS.with(|v| v.set(0));
    crate::serialize::inplace_review::reset();
    let mut s = DeltaSerializer::with_mtu(black_box(case.mtu));
    let mut trace = Trace {
        states: [inventory(&s); 17],
        decisions: [false; 16],
        attempted: 0,
        wire: [0; 65536],
        wire_len: 0,
        calls: 0,
        paths: [0; 4],
    };
    assert!(case.requests.len() <= trace.decisions.len());
    for request in &case.requests {
        let accepted = match black_box(request) {
            Request::Node { id, port } => s.try_add_node(
                ChitchatId::new(id.clone(), 1, ([127, 0, 0, 1], *port).into()),
                0,
                0,
            ),
            Request::Kv {
                key,
                value,
                version,
            } => s.try_add_kv(key, VersionedValue::for_test(value, *version)),
            Request::Max { version } => s.try_set_max_version(*version),
        };
        trace.decisions[trace.attempted] = accepted;
        trace.attempted += 1;
        trace.states[trace.attempted] = inventory(&s);
        if !accepted {
            break;
        }
    }
    let delta = s.finish();
    let wire = black_box(delta.serialize_to_vec());
    assert!(wire.len() <= trace.wire.len());
    trace.wire[..wire.len()].copy_from_slice(&wire);
    trace.wire_len = wire.len();
    drop(black_box(wire));
    drop(black_box(delta));
    trace.calls = COMPRESSIONS.with(Cell::get);
    trace.paths = crate::serialize::inplace_review::take();
    trace
}
fn hex(trace: &Trace) -> String {
    trace.wire[..trace.wire_len]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
fn fixed_caller_semantics_control() {
    for case in table().cases {
        let trace = run(&case);
        assert!(trace.attempted > 0);
        let decisions = &trace.decisions[..trace.attempted];
        assert!(decisions[..decisions.len() - 1].iter().all(|v| *v));
        if !decisions[decisions.len() - 1] {
            assert_eq!(decisions.iter().filter(|v| !**v).count(), 1);
        } else {
            assert_eq!(trace.attempted, case.requests.len());
        }
        let mut view = &trace.wire[..trace.wire_len];
        let delta = Delta::deserialize(&mut view).unwrap();
        assert!(view.is_empty());
        assert_eq!(delta.serialize_to_vec(), &trace.wire[..trace.wire_len]);
        if CALLER_VARIANT != "B" {
            assert!(trace.wire_len <= case.mtu, "{}", case.name);
        }
    }
}
#[test]
fn fixed_caller_capacity_control() {
    for case in table().cases {
        let trace = run(&case);
        assert_eq!(trace, run(&case));
        if case.name.ends_with("/large-then-small") {
            assert!(trace.attempted >= 2 && trace.decisions[0] && trace.decisions[1]);
            if case.mtu > 32768 && CALLER_VARIANT != "B" {
                assert!(
                    trace.paths[1] > 0,
                    "ambiguous admission path must be covered"
                );
            }
        }
        for state in &trace.states[..=trace.attempted] {
            assert!(state.len.iter().zip(state.capacity).all(|(n, c)| *n <= c));
        }
    }
}
#[test]
fn fixed_caller_observations() {
    let mut rows = Vec::new();
    for case in table().cases {
        let original = run(&case);
        let mut records = Vec::new();
        for _ in 0..2 {
            let (trace, meter) = stateful_capture(|| run(black_box(&case)));
            assert_eq!(trace, original);
            records.push(json!({"meter":meter,"states":&trace.states[..=trace.attempted],"decisions":&trace.decisions[..trace.attempted],
                "compressions":trace.calls,"paths":trace.paths,"wire":hex(&trace)}));
        }
        assert_eq!(records[0], records[1]);
        rows.push(json!({"input":case,"records":records}));
    }
    println!(
        "HC074_CALLER:{}",
        json!({"seed":0x0741028_u64,"variant":CALLER_VARIANT,"rows":rows,
        "scope":"actual fixed-MTU DeltaSerializer through final Delta serialization/drop","CPU_qualified":false,"RSS_qualified":false})
    );
}
