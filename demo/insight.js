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
    return { key: key.key, cells, majority, agree: counts.size <= 1 && !counts.has(null) };
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

// Human label for the auto-play interval (milliseconds per step).
export function speedLabel(intervalMs) {
  const ms = Number(intervalMs);
  if (!Number.isFinite(ms) || ms <= 0) {
    return "—";
  }
  if (ms >= 1000) {
    return `${(ms / 1000).toFixed(1)} s/step`;
  }
  return `${Math.round(ms)} ms/step`;
}

// Positions (0–100 %) of timeline events over a scrubber that spans
// [base, max]. Events outside the range are dropped; several events on one
// step collapse to one mark whose kind follows the most important event
// (verdict > leader > action) and whose text lists them all.
export function timelineMarks(events, base, max) {
  const span = max - base;
  if (!(span > 0)) {
    return [];
  }
  const rank = { verdict: 3, leader: 2, action: 1 };
  const byStep = new Map();
  for (const event of events || []) {
    if (event.step < base || event.step > max) {
      continue;
    }
    const mark = byStep.get(event.step);
    if (!mark) {
      byStep.set(event.step, { step: event.step, kind: event.kind, texts: [event.text] });
      continue;
    }
    mark.texts.push(event.text);
    if ((rank[event.kind] ?? 0) > (rank[mark.kind] ?? 0)) {
      mark.kind = event.kind;
    }
  }
  return [...byStep.values()]
    .sort((a, b) => a.step - b.step)
    .map((mark) => ({
      step: mark.step,
      kind: mark.kind,
      percent: ((mark.step - base) / span) * 100,
      text: `step ${mark.step}: ${mark.texts.join("; ")}`,
    }));
}

// What changed between two snapshots of the same run: per-node election and
// log fields, link states, per-node key checksums, the leader, and committed
// entries. Only engine fields are compared; the step counter itself is not a
// "change". Returns null when there is nothing to compare against.
const NODE_FIELDS = [
  ["status", (node) => nodeStatus(node)],
  ["term", (node) => node.term],
  ["commit", (node) => node.commit_index],
  ["applied", (node) => node.applied_index],
];

export function stepChanges(previous, next) {
  if (!previous || !next || previous.seed !== next.seed) {
    return null;
  }
  const before = new Map((previous.nodes || []).map((node) => [node.id, node]));
  const nodes = new Map();
  for (const node of next.nodes || []) {
    const old = before.get(node.id);
    if (!old) {
      nodes.set(node.id, [{ field: "added", from: null, to: nodeStatus(node) }]);
      continue;
    }
    const fields = [];
    for (const [field, read] of NODE_FIELDS) {
      const from = read(old);
      const to = read(node);
      if (from !== to && from !== undefined && to !== undefined) {
        fields.push({ field, from, to });
      }
    }
    if (fields.length) {
      nodes.set(node.id, fields);
    }
  }
  const oldLinks = new Map((previous.links || []).map((link) => [`${link.from}>${link.to}`, link.state]));
  const links = (next.links || [])
    .filter((link) => oldLinks.has(`${link.from}>${link.to}`) && oldLinks.get(`${link.from}>${link.to}`) !== link.state)
    .map((link) => ({ from: link.from, to: link.to, before: oldLinks.get(`${link.from}>${link.to}`), after: link.state }));
  const versions = (snapshot) => {
    const map = new Map();
    for (const key of snapshot.keys || []) {
      for (const replica of key.replicas || []) {
        map.set(`${key.key}|${replica.node_id}`, replica.version ?? null);
      }
    }
    return map;
  };
  const oldVersions = versions(previous);
  const keys = [];
  for (const [id, version] of versions(next)) {
    const old = oldVersions.has(id) ? oldVersions.get(id) : null;
    if (String(old) !== String(version)) {
      const [key, node] = id.split("|");
      keys.push({ key, node, before: old, after: version });
    }
  }
  const leaderBefore = leaderOf(previous);
  const leaderAfter = leaderOf(next);
  const leader = leaderBefore !== leaderAfter ? { from: leaderBefore, to: leaderAfter } : null;
  const committedBefore = previous.progress?.committed_entries;
  const committedAfter = next.progress?.committed_entries;
  const committed =
    committedBefore !== committedAfter && committedAfter !== undefined
      ? { from: committedBefore ?? 0, to: committedAfter }
      : null;

  const summary = [];
  if (leader) {
    summary.push(leader.to ? `leader ${leader.from ?? "none"} → ${leader.to}` : `leader ${leader.from} lost`);
  }
  for (const [id, fields] of nodes) {
    summary.push(
      `${id}: ${fields
        .map((change) => (change.field === "added" ? "added" : `${change.field} ${change.from} → ${change.to}`))
        .join(", ")}`,
    );
  }
  for (const link of links) {
    summary.push(`link ${link.from} → ${link.to} ${link.before} → ${link.after}`);
  }
  if (committed) {
    summary.push(`committed ${committed.from} → ${committed.to}`);
  }
  if (keys.length) {
    summary.push(`${keys.length} key replica checksum(s) changed`);
  }
  return { nodes, links, keys, leader, committed, summary };
}

// What the violation card shows: the engine's own invariant name and detail,
// where it was first reported, and which node ids the detail text names (the
// engine does not attach node ids to a violation, so nothing else is implied).
export function violationFocus(snapshot) {
  if (!snapshot || snapshot.verdict?.status === "holding" || !snapshot.verdict) {
    return null;
  }
  const detail = String(snapshot.verdict.detail ?? "");
  const named = (snapshot.nodes || [])
    .map((node) => node.id)
    .filter((id) => new RegExp(`(^|[^A-Za-z0-9_-])${escapeRegExp(id)}($|[^A-Za-z0-9_-])`).test(detail));
  const first = snapshot.first_violation_step;
  return {
    invariant: String(snapshot.verdict.invariant ?? "invariant"),
    detail,
    step: snapshot.step,
    firstStep: first === null || first === undefined ? snapshot.step : first,
    nodes: named,
  };
}

function escapeRegExp(text) {
  return String(text).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// Rows for the node popover: engine fields only, plus how far its commit index
// trails the current leader's (when there is one).
export function nodeDetails(snapshot, id) {
  const node = (snapshot?.nodes || []).find((candidate) => candidate.id === id);
  if (!node) {
    return null;
  }
  const leaderId = leaderOf(snapshot);
  const leader = (snapshot.nodes || []).find((candidate) => candidate.id === leaderId);
  const rows = [
    ["status", nodeStatus(node)],
    ["term", node.term],
    ["voted for", node.voted_for ?? "—"],
    ["votes received", node.votes_received],
    ["commit index", node.commit_index],
    ["applied index", node.applied_index],
  ];
  if (leader && leader.id !== node.id) {
    const lag = Number(leader.commit_index) - Number(node.commit_index);
    rows.push(["behind leader", lag > 0 ? `${lag} entr${lag === 1 ? "y" : "ies"}` : "caught up"]);
  }
  if (node.region || node.zone) {
    rows.push(["placement", [node.region, node.zone].filter(Boolean).join(" / ")]);
  }
  return { id: node.id, status: nodeStatus(node), crashed: Boolean(node.crashed), disabled: Boolean(node.disabled), rows };
}

// Hue (0–359) for a stored checksum, so equal checksums share a colour in the
// keys heatmap. Missing replicas have no hue.
export function versionHue(version) {
  if (version === null || version === undefined) {
    return null;
  }
  const hex = shortVersion(version);
  let hash = 0;
  for (const char of hex) {
    hash = (hash * 31 + char.charCodeAt(0)) % 3600;
  }
  return Math.round(hash / 10) % 360;
}

const EVENT_ICONS = { action: "▸", leader: "★", verdict: "✕" };

export function eventIcon(event) {
  if (event.kind === "verdict" && /holds again/.test(event.text)) {
    return "✓";
  }
  return EVENT_ICONS[event.kind] ?? "•";
}

// Timeline events grouped by step (ascending), each item with its icon; a
// group is `current` at the visible step and `future` past it (rewound view).
export function groupEvents(events, currentStep) {
  const groups = new Map();
  for (const event of events || []) {
    if (!groups.has(event.step)) {
      groups.set(event.step, []);
    }
    groups.get(event.step).push({ kind: event.kind, text: event.text, icon: eventIcon(event) });
  }
  return [...groups.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([step, items]) => ({
      step,
      items,
      current: step === currentStep,
      future: step > currentStep,
    }));
}

// Keyboard shortcut for a keydown, or null. `target` is "field" (typing),
// "control" (a focused button, tab, slider, or checkbox, which owns Space and
// the arrow keys), or "page".
export function shortcutFor(event, target = "page") {
  if (!event || event.ctrlKey || event.metaKey || event.altKey || target === "field") {
    return null;
  }
  const key = event.key;
  if (key === "?") {
    return "help";
  }
  if (key === ".") {
    return "step";
  }
  if (key === "f" || key === "F") {
    return "fit";
  }
  if (target === "control") {
    return null;
  }
  if (key === " " || key === "Spacebar") {
    return "toggle-play";
  }
  if (key === "ArrowLeft") {
    return event.shiftKey ? "back-10" : "back";
  }
  if (key === "ArrowRight") {
    return event.shiftKey ? "forward-10" : "forward";
  }
  return null;
}

// ---------------------------------------------------------------------------
// Guide outcomes. After "Do it" the guide states what the engine actually
// did, from the snapshot alone; `ok` is false when the step's claim did not
// happen, and the guide says so instead of moving on as if it had.
// ---------------------------------------------------------------------------
function plural(count, noun) {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

export function quorumSize(snapshot) {
  return Math.floor((snapshot.nodes || []).length / 2) + 1;
}

export function predictionHolds(when, snapshot) {
  switch (when) {
    case "leader":
      return leaderOf(snapshot) !== null;
    case "no_leader":
      return leaderOf(snapshot) === null;
    default:
      throw new Error(`unknown guide prediction '${when}'`);
  }
}

export function tourOutcome(report, snapshot, ctx = {}) {
  const total = (snapshot.nodes || []).length;
  const live = liveNodeIds(snapshot).length;
  const quorum = quorumSize(snapshot);
  const leader = leaderOf(snapshot);
  const committed = snapshot.progress?.committed_entries ?? 0;
  switch (report?.kind) {
    case "leader": {
      if (!leader) return { ok: false, text: `No leader at step ${snapshot.step}.` };
      const node = snapshot.nodes.find((candidate) => candidate.id === leader);
      const previous = ctx.before ? leaderOf(ctx.before) : null;
      const change = previous && previous !== leader ? ` (was ${previous})` : "";
      return { ok: true, text: `Leader ${leader}${change}, term ${node.term}, at step ${snapshot.step}.` };
    }
    case "subscribed": {
      const sub = (snapshot.subscribers || []).find(
        (candidate) => candidate.client_id === report.client && candidate.namespace === report.ns,
      );
      return sub
        ? { ok: true, text: `${report.client} is subscribed to ${report.ns}, attached to ${sub.connected_node}.` }
        : { ok: false, text: `${report.client} is not subscribed to ${report.ns}.` };
    }
    case "delivered": {
      const fullKey = `${report.ns}:${report.key}`;
      const sub = (snapshot.subscribers || []).find(
        (candidate) => candidate.client_id === report.client && candidate.namespace === report.ns,
      );
      const event = sub?.last_event;
      if (!event || event.key !== fullKey) {
        return { ok: false, text: `${report.key} has not been delivered to ${report.client} yet.` };
      }
      const replicas = (snapshot.keys || []).find((key) => key.key === fullKey)?.replicas ?? [];
      const holding = replicas.filter((replica) => replica.version === event.version).length;
      return {
        ok: true,
        text: `${report.key} committed at index ${event.commit_index} and delivered to ${report.client} at step ${event.delivered_at_step}; ${holding} of ${replicas.length} replicas hold it.`,
      };
    }
    case "crashed": {
      const node = ctx.crashed;
      const down = (snapshot.nodes || []).find((candidate) => candidate.id === node)?.crashed;
      if (!down) return { ok: false, text: `${node ?? "The node"} is not crashed.` };
      const verdict = live >= quorum ? "still a quorum" : "not a quorum";
      return { ok: true, text: `${node} crashed: ${live} of ${total} nodes live, quorum is ${quorum} (${verdict}).` };
    }
    case "no_leader": {
      if (leader) return { ok: false, text: `${leader} is still leader at step ${snapshot.step}.` };
      const unavailable = (snapshot.consistency || []).filter((level) => !level.satisfiable).map((level) => level.level);
      const levels = unavailable.length ? ` Unavailable levels: ${unavailable.join(", ")}.` : "";
      const before = ctx.before?.progress?.committed_entries;
      const progress =
        before === undefined || before === committed
          ? `Committed entries: ${committed}.`
          : `Committed entries ${before} → ${committed} (written before the quorum was lost).`;
      return {
        ok: true,
        text: `No leader: ${live} of ${total} nodes live, a quorum needs ${quorum}. ${progress}${levels}`,
      };
    }
    case "agree": {
      const agreement = keyAgreement(snapshot);
      return agreement.agree
        ? { ok: true, text: `All live replicas agree on ${plural(agreement.keys.length, "sampled key")} at step ${snapshot.step}.` }
        : { ok: false, text: `Replicas still differ at step ${snapshot.step}.` };
    }
    case "committed": {
      const before = ctx.before?.progress?.committed_entries ?? committed;
      return { ok: true, text: `Committed entries ${before} → ${committed} by step ${snapshot.step}.` };
    }
    case "diverged": {
      const differing = keyAgreement(snapshot).keys.filter((key) => !key.agree).length;
      const sampled = (snapshot.keys || []).length;
      return {
        ok: true,
        text: `${differing} of ${plural(sampled, "sampled key")} ${differing === 1 ? "differs" : "differ"} between replicas at step ${snapshot.step}.`,
      };
    }
    default:
      throw new Error(`unknown guide report '${report?.kind}'`);
  }
}
