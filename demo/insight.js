// Pure view-model helpers for the simulator lab. Everything here is derived
// from engine snapshots and replay scripts only; nothing is invented. Kept free
// of DOM access so scripts/check-static.mjs can unit-test it under Node.

export const VOPR_COMMAND = "cargo run -p hydracache-sim --bin vopr --";

// A run needs the replay script (not just seed + steps) once anything beyond
// the seed shaped it: an intervention, a mode change, or a curated scenario.
export function needsReplayScript(snapshot) {
  return Boolean(snapshot?.intervention_count > 0 || snapshot?.active_scenario);
}

export function scriptFileName(snapshot) {
  return `hydracache-replay-seed-${snapshot.seed}-step-${snapshot.step}.json`;
}

export function voprScriptCommand(fileName) {
  return `${VOPR_COMMAND} --script ${fileName}`;
}

export function voprSeedCommand(seed, steps) {
  return `${VOPR_COMMAND} --seed ${seed} --steps ${steps}`;
}

// Verdict line shown in the header. A violation always names how to reproduce
// *this* run: the plain seed command is only offered when it is equivalent.
export function verdictView(snapshot) {
  const first = snapshot.first_violation_step;
  if (snapshot.verdict.status === "holding") {
    if (first === null || first === undefined) {
      return { kind: "ok", text: `invariants hold @ seed ${snapshot.seed}` };
    }
    return {
      kind: "warn",
      text: `invariants hold now @ seed ${snapshot.seed}; a violation was first reported at step ${first}`,
    };
  }
  const reproduce = needsReplayScript(snapshot)
    ? `reproduce: Copy reproducer (replay link) or Download script, then ${voprScriptCommand(scriptFileName(snapshot))}`
    : `reproduce: ${voprSeedCommand(snapshot.seed, snapshot.step)}`;
  const firstText = first === null || first === undefined ? "" : ` (first at step ${first})`;
  return {
    kind: "bad",
    text: `violation: ${snapshot.verdict.invariant} @ seed ${snapshot.seed}, step ${snapshot.step}${firstText}; ${reproduce}`,
  };
}

export function liveNodeIds(snapshot) {
  return (snapshot.nodes || [])
    .filter((node) => !node.crashed && !node.disabled)
    .map((node) => node.id);
}

export function leaderOf(snapshot) {
  return (snapshot.nodes || []).find((node) => node.vote_state === "leader")?.id ?? null;
}

// Per-key replica comparison from the engine's stored checksums. Agreement is
// judged over live (not crashed, not disabled) nodes; a live node without the
// key counts as disagreeing because it would serve a miss.
export function keyAgreement(snapshot) {
  const nodes = (snapshot.nodes || []).map((node) => node.id);
  const live = new Set(liveNodeIds(snapshot));
  const keys = (snapshot.keys || []).map((key) => {
    const byNode = new Map(key.replicas.map((replica) => [replica.node_id, replica.version]));
    const liveVersions = nodes.filter((id) => live.has(id)).map((id) => byNode.get(id) ?? null);
    const counts = new Map();
    for (const version of liveVersions) {
      counts.set(version, (counts.get(version) ?? 0) + 1);
    }
    let majority = null;
    let best = -1;
    for (const [version, count] of counts) {
      if (count > best) {
        majority = version;
        best = count;
      }
    }
    const cells = nodes.map((id) => {
      const version = byNode.get(id) ?? null;
      return {
        node: id,
        version,
        live: live.has(id),
        diverged: live.has(id) && version !== majority,
      };
    });
    return { key: key.key, cells, agree: counts.size <= 1 && !counts.has(null) };
  });
  return { keys, agree: keys.every((key) => key.agree) };
}

export function shortVersion(version) {
  if (version === null || version === undefined) {
    return "—";
  }
  return BigInt.asUintN(64, BigInt(version)).toString(16).padStart(16, "0").slice(-6);
}

export function describeAction(action) {
  switch (action?.type) {
    case "crash":
      return `crash ${action.node}`;
    case "restart":
      return `restart ${action.node}`;
    case "isolate":
      return `isolate ${action.node}`;
    case "rejoin":
      return `rejoin ${action.node}`;
    case "disable":
      return `disable ${action.node}`;
    case "enable":
      return `enable ${action.node}`;
    case "add_node":
      return "add node";
    case "partition":
      return `partition ${action.from} -> ${action.to}`;
    case "heal":
      return `heal ${action.from} -> ${action.to}`;
    case "drop_next":
      return `drop next ${action.from} -> ${action.to}`;
    case "delay_next":
      return `delay next ${action.from} -> ${action.to} by ${action.millis} ms`;
    case "workload":
      return action.enabled ? "workload on" : "workload off";
    case "subscribe":
      return `subscribe ${action.client}@${action.ns}`;
    case "push_event":
      return `push ${action.ns}/${action.key} from ${action.client}`;
    case "mode_change":
      return `mode ${action.mode}`;
    case "step":
      return `run ${action.n} steps`;
    default:
      return String(action?.type ?? "action");
  }
}

// Timeline events: recorded interventions from the replay script plus what the
// page itself observed (leader changes, verdict transitions). Observations are
// labelled as such because steps run inside a single action are not observed.
export function timelineEvents(script, observations) {
  const events = [];
  for (const action of script?.actions || []) {
    events.push({ step: action.at_step, kind: "action", text: describeAction(action) });
  }
  let previous = null;
  for (const [step, observed] of [...observations.entries()].sort((a, b) => a[0] - b[0])) {
    if (previous && previous.leader !== observed.leader) {
      events.push({
        step,
        kind: "leader",
        text: observed.leader ? `leader is ${observed.leader}` : "no leader",
      });
    }
    if (previous && previous.verdict !== observed.verdict) {
      events.push({
        step,
        kind: "verdict",
        text: observed.verdict === "holding" ? "verdict holds again" : `violation: ${observed.invariant}`,
      });
    }
    previous = observed;
  }
  return events.sort((a, b) => a.step - b.step || (a.kind === "action" ? -1 : 1));
}

// The replay script that rebuilds the recorded run at `step`: interventions at
// or before that step, resumed exactly there.
export function scriptAtStep(script, step) {
  return {
    ...script,
    actions: (script.actions || []).filter((action) => action.at_step <= step),
    target_step: step,
  };
}

// Display status of a node: crash/disable override the election state.
export function nodeStatus(node) {
  if (node.crashed) {
    return "crashed";
  }
  if (node.disabled) {
    return "disabled";
  }
  return String(node.vote_state || node.role || "unknown");
}

// Text alternative for the cluster graph (screen readers, tests): the same
// engine facts the picture encodes.
export function graphSummary(snapshot) {
  const nodes = (snapshot.nodes || []).map((node) => `${node.id} ${nodeStatus(node)}`);
  const states = new Map();
  for (const link of snapshot.links || []) {
    if (link.state !== "up") {
      states.set(link.state, (states.get(link.state) ?? 0) + 1);
    }
  }
  const links = states.size
    ? [...states].map(([state, count]) => `${count} ${state}`).join(", ")
    : "all up";
  const clients = (snapshot.clients || []).length;
  const subscribers = (snapshot.subscribers || []).length;
  return (
    `${nodes.length} nodes: ${nodes.join(", ")}. Directed links: ${links}. ` +
    `${clients} client(s), ${subscribers} subscriber(s), ${(snapshot.in_flight || []).length} message(s) in flight.`
  );
}
