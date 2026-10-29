//! Synthetic observer cost only. No live guard, manager, product or admission.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::hint::black_box;
use std::time::Instant;

// Import the exact current observers, not a faster local reimplementation.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagnosticOperation {
    Heartbeat,
}
#[allow(dead_code)]
#[path = "../../long-run-supervisor-074/src/diagnostic_validation_trace.rs"]
mod trace;
use trace::kernel_walk as kernel;

const SEED: u64 = 0x074c051e;
const REPEATS: usize = 3;
const KINDS: [kernel::Kind; 10] = [
    kernel::Kind::PacketContext,
    kernel::Kind::Sender,
    kernel::Kind::Alive,
    kernel::Kind::Projection,
    kernel::Kind::Origin,
    kernel::Kind::Boot,
    kernel::Kind::Namespace,
    kernel::Kind::Sample,
    kernel::Kind::WorkerOrigin,
    kernel::Kind::Document,
];
const ORDERS: [[&str; 3]; 6] = [
    ["inactive", "kernel", "rounds"],
    ["rounds", "kernel", "inactive"],
    ["kernel", "inactive", "rounds"],
    ["rounds", "inactive", "kernel"],
    ["kernel", "rounds", "inactive"],
    ["inactive", "rounds", "kernel"],
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Graph {
    calls: [u64; 10],
    roots: [u64; 10],
    edges: [[u64; 10]; 10],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema: String,
    seed: u64,
    rounds: usize,
    input_source: String,
    input_inventory_sha256: String,
    steps: Vec<Graph>,
}
#[derive(Clone, Copy)]
enum Event {
    Enter(usize),
    Exit,
}
struct Plan {
    graphs: Vec<Graph>,
    tapes: Vec<Vec<Event>>,
    depth: usize,
}
fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn visit(i: usize, graph: &Graph, states: &mut [u8; 10]) -> Result<(), String> {
    require(
        states[i] != 1,
        "cyclic topology cannot reconstruct a bounded forest",
    )?;
    if states[i] == 2 {
        return Ok(());
    }
    states[i] = 1;
    for j in 0..10 {
        if graph.edges[i][j] != 0 {
            visit(j, graph, states)?;
        }
    }
    states[i] = 2;
    Ok(())
}
fn validate(graph: &Graph) -> Result<(), String> {
    require(
        graph
            .calls
            .iter()
            .chain(graph.roots.iter())
            .chain(graph.edges.iter().flatten())
            .all(|n| *n <= 25_000),
        "graph count exceeds bound",
    )?;
    require(
        graph.calls.iter().sum::<u64>() <= 25_000,
        "per-round graph exceeds fixed bound",
    )?;
    for i in 0..10 {
        require(
            graph.calls[i] == graph.roots[i] + graph.edges.iter().map(|e| e[i]).sum::<u64>(),
            "root/edge count differs",
        )?;
        for count in graph.edges[i] {
            require(
                if graph.calls[i] == 0 {
                    count == 0
                } else {
                    count % graph.calls[i] == 0
                },
                "nonuniform arity is not recorded; refuse reconstruction",
            )?;
        }
    }
    let mut states = [0; 10];
    for i in 0..10 {
        visit(i, graph, &mut states)?;
    }
    Ok(())
}
fn emit(i: usize, depth: usize, graph: &Graph, tape: &mut Vec<Event>, max_depth: &mut usize) {
    *max_depth = (*max_depth).max(depth);
    tape.push(Event::Enter(i));
    for j in 0..10 {
        for _ in 0..graph.edges[i][j] / graph.calls[i] {
            emit(j, depth + 1, graph, tape, max_depth);
        }
    }
    tape.push(Event::Exit);
}
impl Plan {
    fn new(input: Input) -> Result<Self, String> {
        require(
            input.schema == "observer-kernel-topology-074-v1"
                && input.seed == SEED
                && input.rounds == 46,
            "fixed workload identity differs",
        )?;
        require(
            input.input_source == "c1aa639990c4d2434a896ab7e90d93a0f8ce4b8a"
                && input.input_inventory_sha256
                    == "91a6e589235f2552ae9335680f47d4a20a418efeefcc3bc9fd2a05ba96f12afe",
            "frozen input identity differs",
        )?;
        require(input.steps.len() == 8, "eight ordered steps required")?;
        let mut tapes = Vec::new();
        let mut depth = 0;
        for graph in &input.steps {
            validate(graph)?;
            let mut tape = Vec::new();
            for i in 0..10 {
                for _ in 0..graph.roots[i] {
                    emit(i, 1, graph, &mut tape, &mut depth);
                }
            }
            let mut actual = [0; 10];
            for event in &tape {
                if let Event::Enter(i) = event {
                    actual[*i] += 1;
                }
            }
            require(actual == graph.calls, "canonical tape count differs")?;
            tapes.push(tape);
        }
        require(depth <= 10, "topology depth exceeds bound")?;
        Ok(Self {
            graphs: input.steps,
            tapes,
            depth,
        })
    }
}
#[derive(Clone, Copy)]
enum Mode {
    Inactive,
    Kernel,
    Rounds,
}
fn mode(name: &str) -> Mode {
    match name {
        "inactive" => Mode::Inactive,
        "kernel" => Mode::Kernel,
        "rounds" => Mode::Rounds,
        _ => unreachable!(),
    }
}
fn walk(plan: &Plan) -> (u64, [u64; 10]) {
    let mut state = SEED;
    let mut counts = [0; 10];
    let mut stack = Vec::with_capacity(plan.depth);
    for _ in 0..46 {
        for (step, tape) in plan.tapes.iter().enumerate() {
            let round = trace::round_cost::span(step);
            for event in tape {
                match *event {
                    Event::Enter(i) => {
                        stack.push(kernel::span(KINDS[black_box(i)]));
                        counts[i] += 1;
                        state = state.rotate_left(7) ^ (black_box(i as u64) + 74);
                    }
                    Event::Exit => {
                        drop(stack.pop().unwrap());
                    }
                }
            }
            round.complete();
        }
    }
    assert!(stack.is_empty());
    (black_box(state), counts)
}
fn cpu() -> u128 {
    let mut value: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut value) },
        0
    );
    assert!(value.tv_sec >= 0 && (0..1_000_000_000).contains(&value.tv_nsec));
    value.tv_sec as u128 * 1_000_000_000 + value.tv_nsec as u128
}
fn timed<T>(call: impl FnOnce() -> T) -> (T, Value) {
    let before = cpu();
    let started = Instant::now();
    let value = call();
    let wall = started.elapsed().as_nanos();
    let process = cpu().checked_sub(before).expect("CPU counter reversed");
    (
        value,
        json!({"wall_ns":wall,"parent_process_cpu_ns":process}),
    )
}
fn counts_match(plan: &Plan, report: &kernel::Report) {
    for i in 0..10 {
        assert_eq!(
            report.nodes[i].calls,
            plan.graphs.iter().map(|g| g.calls[i] * 46).sum::<u64>()
        );
        assert_eq!(
            report.roots[i],
            plan.graphs.iter().map(|g| g.roots[i] * 46).sum::<u64>()
        );
        for j in 0..10 {
            assert_eq!(
                report.edges[i][j].calls,
                plan.graphs.iter().map(|g| g.edges[i][j] * 46).sum::<u64>()
            );
        }
        assert_eq!(
            report.nodes[i].inclusive_ns,
            report.nodes[i].outside_children_ns
                + report.edges[i].iter().map(|e| e.inclusive_ns).sum::<u128>()
        );
    }
}
fn round_counts_match(plan: &Plan, rows: &[Value]) {
    assert_eq!(rows.len(), 8);
    for (step, row) in rows.iter().enumerate() {
        assert_eq!(row["step"], step);
        assert_eq!(row["calls"], 46);
        assert_eq!(row["completed"], 46);
        for i in 0..10 {
            assert_eq!(row["kernel"][i]["calls"], plan.graphs[step].calls[i] * 46);
            assert_eq!(row["roots"][i], plan.graphs[step].roots[i] * 46);
            for j in 0..10 {
                let sum = row["edges"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|e| e["parent"] == i && e["child"] == j)
                    .map(|e| e["calls"].as_u64().unwrap())
                    .sum::<u64>();
                assert_eq!(sum, plan.graphs[step].edges[i][j] * 46);
            }
        }
    }
}
fn sample(plan: &Plan, mode: Mode) -> Value {
    let round = matches!(mode, Mode::Rounds).then(trace::round_cost::Capture::new);
    let ((body, report), walk_time) = timed(|| match mode {
        Mode::Inactive => (walk(plan), None),
        _ => {
            let (body, report) = kernel::capture(|| walk(plan));
            (body, Some(report))
        }
    });
    for i in 0..10 {
        assert_eq!(
            body.1[i],
            plan.graphs.iter().map(|g| g.calls[i] * 46).sum::<u64>()
        );
    }
    if let Some(report) = &report {
        counts_match(plan, report);
    }
    // Aggregation and serialization are timed separately, outside the walk.
    let ((rows, payload), aggregation_time) = timed(|| {
        let rows = round.map(|r| r.finish()).unwrap_or_default();
        let nodes:Vec<_>=report.as_ref().map(|r|r.nodes.iter().map(|n|json!({"calls":n.calls,"inclusive_ns":n.inclusive_ns,"outside_children_ns":n.outside_children_ns})).collect()).unwrap_or_default();
        let payload = json!({"kernel":nodes,"round_steps":rows,"synthetic":true});
        (rows, payload)
    });
    if matches!(mode, Mode::Rounds) {
        round_counts_match(plan, &rows);
    }
    let (encoded, serialization_time) = timed(|| serde_json::to_vec(&payload).unwrap());
    let decoded = serde_json::from_slice::<Value>(&encoded).unwrap();
    assert_eq!(decoded, payload);
    let topology = report.as_ref().map(|r| {
        json!({"roots":r.roots,
        "edge_calls":r.edges.map(|row|row.map(|e|e.calls)),
        "edge_inclusive_ns":r.edges.map(|row|row.map(|e|e.inclusive_ns))})
    });
    json!({"walk":walk_time,"aggregation":aggregation_time,"serialization":serialization_time,
        "checksum":body.0,"calls":body.1,
        "report_bytes":encoded.len(),"report":payload,"observed_topology":topology})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    require(
        args.len() == 7 && args[1] == "--input" && args[3] == "--case" && args[5] == "--source",
        "fixed arguments required",
    )?;
    let case: usize = args[4].parse()?;
    require(
        case < 6 && args[6].len() == 40 && args[6].bytes().all(|b| b.is_ascii_hexdigit()),
        "case/source invalid",
    )?;
    let raw = std::fs::read(&args[2])?;
    require(raw.len() <= 65536, "input exceeds bound")?;
    let plan = Plan::new(serde_json::from_slice(&raw)?)?;
    let mut blocks = Vec::new();
    for name in ORDERS[case] {
        let samples: Vec<_> = (0..REPEATS).map(|_| sample(&plan, mode(name))).collect();
        blocks.push(json!({"mode":name,"samples":samples}));
    }
    let checksum = blocks[0]["samples"][0]["checksum"].clone();
    assert!(blocks.iter().all(|b| b["samples"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["checksum"] == checksum)));
    println!(
        "{}",
        json!({"schema":"observer-topology-control-074-v1","source":args[6],"case":case,
        "order":ORDERS[case],"seed":SEED,"rounds":46,"repeats":REPEATS,"blocks":blocks,
        "scope":"kernel observer plus round snapshots; live/auth observers inactive",
        "canonical_order":true,"original_temporal_order_known":false,"synthetic":true,
        "live_guard_invocations":0,"admission_exercised":false,"native_gate_open":false,
        "guard_or_deadline_changed":false,"server":false,"product":false})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn small() -> Graph {
        let mut graph = Graph {
            calls: [0; 10],
            roots: [0; 10],
            edges: [[0; 10]; 10],
        };
        graph.calls[0] = 1;
        graph.calls[1] = 2;
        graph.roots[0] = 1;
        graph.edges[0][1] = 2;
        graph
    }
    fn input(graph: Graph) -> Input {
        Input {
            schema: "observer-kernel-topology-074-v1".into(),
            seed: SEED,
            rounds: 46,
            input_source: "c1aa639990c4d2434a896ab7e90d93a0f8ce4b8a".into(),
            input_inventory_sha256:
                "91a6e589235f2552ae9335680f47d4a20a418efeefcc3bc9fd2a05ba96f12afe".into(),
            steps: vec![graph; 8],
        }
    }
    #[test]
    fn canonical_plan_keeps_exact_nodes_roots_edges_and_checksum() {
        let plan = Plan::new(input(small())).unwrap();
        for name in ORDERS[0] {
            let result = sample(&plan, mode(name));
            assert_eq!(result["calls"][0], 368);
            assert_eq!(result["calls"][1], 736);
        }
        assert!(kernel::numeric_snapshot().is_none());
    }
    #[test]
    fn malformed_counts_nonuniform_edges_and_cycles_refuse() {
        let mut graph = small();
        graph.calls[1] = 3;
        assert!(validate(&graph).is_err());
        let mut graph = small();
        graph.calls[0] = 2;
        graph.roots[0] = 2;
        graph.calls[1] = 3;
        graph.edges[0][1] = 3;
        assert!(validate(&graph).is_err());
        let mut graph = small();
        graph.calls = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        graph.roots = [0; 10];
        graph.edges[0][1] = 1;
        graph.edges[1][0] = 1;
        assert!(validate(&graph).is_err());
    }
    #[test]
    fn empty_graph_keeps_46_complete_rounds_without_kernel_calls() {
        let plan = Plan::new(input(Graph {
            calls: [0; 10],
            roots: [0; 10],
            edges: [[0; 10]; 10],
        }))
        .unwrap();
        let result = sample(&plan, Mode::Rounds);
        assert_eq!(result["calls"], serde_json::to_value([0_u64; 10]).unwrap());
    }
    #[test]
    fn fixed_seed_rounds_and_schema_are_not_tunable() {
        let mut value = input(small());
        value.rounds = 47;
        assert!(Plan::new(value).is_err());
        let mut value = input(small());
        value.seed += 1;
        assert!(Plan::new(value).is_err());
        let mut value = input(small());
        value.steps.pop();
        assert!(Plan::new(value).is_err());
    }
    #[test]
    fn six_orders_balance_each_position() {
        for name in ORDERS[0] {
            for position in 0..3 {
                assert_eq!(ORDERS.iter().filter(|r| r[position] == name).count(), 2);
            }
        }
    }
}
