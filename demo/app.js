import {
  readInitialState,
  reproducerCommand,
  snapshotHash,
  writeUrlState,
} from "./share.js";
import { SCENARIOS } from "./scenarios.js";

const SVG_NS = "http://www.w3.org/2000/svg";
const CONSISTENCY_LEVELS = ["ONE", "LOCAL_QUORUM", "QUORUM", "EACH_QUORUM", "ALL"];

const state = {
  SimHandle: null,
  engine: "wasm",
  apiBase: "",
  sim: null,
  snapshot: null,
  selectedLink: null,
  scenario: "default",
  timer: null,
  playing: false,
  tickPending: false,
  seenNodes: new Set(),
  packets: [],
  animationFrame: null,
  packetStart: 0,
};

const el = {
  banner: document.querySelector("#engine-banner"),
  verdict: document.querySelector("#verdict"),
  electionSourceChip: document.querySelector("#election-source-chip"),
  seedInput: document.querySelector("#seed-input"),
  scenario: document.querySelector("#scenario-select"),
  mode: document.querySelector("#mode-select"),
  interventionStatus: document.querySelector("#intervention-status"),
  loadScenario: document.querySelector("#load-scenario"),
  reset: document.querySelector("#reset-button"),
  step: document.querySelector("#step-button"),
  play: document.querySelector("#play-button"),
  copy: document.querySelector("#copy-reproducer"),
  copyStatus: document.querySelector("#copy-status"),
  workload: document.querySelector("#workload-toggle"),
  speed: document.querySelector("#speed-input"),
  manualClient: document.querySelector("#manual-client"),
  manualNs: document.querySelector("#manual-ns"),
  manualKey: document.querySelector("#manual-key"),
  manualValue: document.querySelector("#manual-value"),
  subscribe: document.querySelector("#subscribe-button"),
  pushEvent: document.querySelector("#push-event-button"),
  graph: document.querySelector("#cluster-graph"),
  selectedLink: document.querySelector("#selected-link"),
  progress: document.querySelector("#progress-panel"),
  hash: document.querySelector("#snapshot-hash"),
  nodes: document.querySelector("#nodes-panel"),
  addNode: document.querySelector("#add-node-button"),
  signals: document.querySelector("#signals-panel"),
  clients: document.querySelector("#clients-panel"),
  subscribers: document.querySelector("#subscribers-panel"),
  consistency: document.querySelector("#consistency-panel"),
  keys: document.querySelector("#keys-panel"),
  linkActions: Array.from(document.querySelectorAll("[data-action]")),
};

async function boot() {
  bindPreferenceState();
  const initial = readInitialState(window.location.search);
  state.engine = initial.engine;
  state.apiBase = initial.apiBase;
  state.scenario = initial.scenario;
  el.seedInput.value = String(initial.seed);
  populateScenarios(initial.scenario);
  bindEvents();
  try {
    if (state.engine === "wasm") {
      const wasm = await import("./pkg/hydracache_sim_wasm.js");
      if (typeof wasm.default === "function") {
        await wasm.default();
      }
      state.SimHandle = wasm.SimHandle;
    }
    if (initial.script) {
      await loadReplayScript(initial.script);
    } else if (initial.scenario === "default") {
      await resetSimulation(initial.steps);
    } else {
      await loadScenario(initial.scenario, initial.steps);
    }
  } catch (error) {
    showEngineError(error);
  }
}

function bindEvents() {
  el.reset.addEventListener("click", () => void resetSimulation());
  el.loadScenario.addEventListener("click", () => void loadScenario(el.scenario.value));
  el.mode.addEventListener("change", async () => {
    await state.sim?.set_mode(el.mode.value);
    await refresh();
  });
  el.step.addEventListener("click", async () => {
    await state.sim?.step();
    await refresh();
  });
  el.play.addEventListener("click", togglePlay);
  el.copy.addEventListener("click", copyReproducer);
  el.subscribe.addEventListener("click", async () => {
    await state.sim?.subscribe(manualClientId(), el.manualNs.value);
    await settleAfterIntervention();
    await refresh();
  });
  el.pushEvent.addEventListener("click", async () => {
    await pushEventFor(manualClientId(), el.manualNs.value);
  });
  el.addNode.addEventListener("click", async () => {
    await state.sim?.add_node();
    await settleAfterIntervention();
    await refresh();
  });
  el.workload.addEventListener("change", async () => {
    await state.sim?.set_workload_enabled(el.workload.checked);
    await refresh();
  });
  el.speed.addEventListener("input", () => {
    if (state.playing) {
      stopPlay();
      startPlay();
    }
  });
  for (const button of el.linkActions) {
    button.addEventListener("click", () => void applyLinkAction(button.dataset.action));
  }
}

async function resetSimulation(steps = 0) {
  if (state.engine === "wasm" && !state.SimHandle) {
    return;
  }
  stopPlay();
  state.scenario = "default";
  el.scenario.value = state.scenario;
  state.selectedLink = null;
  state.sim = await createSimulation(readSeed());
  await state.sim.set_workload_enabled(el.workload.checked);
  if (steps > 0) {
    await state.sim.run(steps);
  }
  await refresh();
}

async function loadScenario(name, targetSteps = null) {
  if (state.engine === "wasm" && !state.SimHandle) {
    return;
  }
  stopPlay();
  state.selectedLink = null;
  state.scenario = name || "default";
  el.scenario.value = state.scenario;
  if (state.scenario === "default") {
    await resetSimulation(Number.isInteger(targetSteps) ? targetSteps : 0);
    return;
  }
  state.sim = await createSimulation(0);
  await state.sim.apply_scenario(state.scenario);
  if (Number.isInteger(targetSteps)) {
    const snapshot = JSON.parse(await state.sim.snapshot_json());
    if (targetSteps > snapshot.step) {
      await state.sim.run(targetSteps - snapshot.step);
    }
  }
  await refresh();
  el.seedInput.value = String(state.snapshot.seed);
}

async function refresh() {
  if (!state.sim) {
    return;
  }
  state.snapshot = JSON.parse(await state.sim.snapshot_json());
  let replayScriptJson = "";
  try {
    replayScriptJson = (await state.sim.replay_script_json?.()) ?? "";
  } catch (_error) {
    replayScriptJson = "";
  }
  writeUrlState(
    window.history,
    state.snapshot,
    state.scenario,
    state.engine,
    state.apiBase,
    replayScriptJson,
  );
  render();
}

function render() {
  const snapshot = state.snapshot;
  el.banner.textContent = `This runs the real hydracache-sim engine, seed ${snapshot.seed}, step ${snapshot.step}. Formation ${snapshot.formation_phase}; election ${snapshot.election_source}. ${snapshot.election_disclosure || "Verdicts are produced by the actual invariant checker."}`;
  renderElectionSource(snapshot);
  el.scenario.value = state.scenario;
  el.mode.value = snapshot.mode || "manual";
  el.interventionStatus.textContent =
    snapshot.intervention_count > 0
      ? `${snapshot.mode} · ${snapshot.intervention_count} replay action(s)`
      : `${snapshot.mode || "manual"} · no interventions`;
  renderVerdict(snapshot);
  syncGraph(snapshot);
  renderSelectedLink();
  renderProgress(snapshot);
  renderNodes(snapshot);
  renderSignals(snapshot);
  renderClients(snapshot);
  renderConsistency(snapshot);
  renderKeys(snapshot);
}

function renderElectionSource(snapshot) {
  const source = snapshot.election_source || "unknown";
  const sourceClass = source
    .replace(/[^a-z0-9]+/gi, "-")
    .replace(/^-|-$/g, "")
    .toLowerCase();
  el.electionSourceChip.className = `source-chip source-${sourceClass || "unknown"}`;
  el.electionSourceChip.textContent = source;
  el.electionSourceChip.title =
    snapshot.election_disclosure || "Election source for this simulator snapshot.";
  el.electionSourceChip.setAttribute("aria-label", `Election source: ${source}`);
}

function renderVerdict(snapshot) {
  el.verdict.className = "verdict";
  if (snapshot.verdict.status === "holding") {
    el.verdict.classList.add("ok");
    el.verdict.textContent = `invariants hold @ seed ${snapshot.seed}`;
  } else {
    el.verdict.classList.add("bad");
    el.verdict.textContent = `violation: ${snapshot.verdict.invariant} @ seed ${snapshot.seed}; ${reproducerCommand(
      snapshot.seed,
      snapshot.step,
    )}`;
  }
}

// ---------------------------------------------------------------------------
// Force-directed, draggable cluster graph (Obsidian-style). Positions persist
// across snapshots; a physics tick (repulsion + edge springs) keeps spacing and
// smoothly pulls a client to a new node when its routing changes; vertices are
// draggable and the canvas pans. DOM is rebuilt only on topology change; visual
// attributes and positions update in place.
// ---------------------------------------------------------------------------
const GRAPH = {
  center: { x: 400, y: 260 },
  nodeRadius: 46,
  entityRadius: 20,
  springNode: 168, // rest length between cluster nodes (more breathing room)
  springEntity: 132, // rest length node <-> client/subscriber (bigger distance)
  repulsion: 52000,
  spring: 0.018,
  damping: 0.85,
  centerPull: 0.006,
  maxVelocity: 26,
};

const graphSim = {
  pos: new Map(), // id -> { x, y, vx, vy, fixed, kind }
  vertices: [], // [{ id, kind }]
  edges: [], // [{ a, b, kind }]
  dom: null,
  topoKey: "",
  view: { tx: 0, ty: 0, scale: 1 },
  drag: null,
  raf: null,
  packets: [],
  pulses: [],
  interactionsBound: false,
};

function resetGraph() {
  if (graphSim.raf !== null) {
    cancelAnimationFrame(graphSim.raf);
    graphSim.raf = null;
  }
  graphSim.pos.clear();
  graphSim.topoKey = "";
  graphSim.view = { tx: 0, ty: 0, scale: 1 };
  graphSim.drag = null;
  graphSim.packets = [];
}

function syncGraph(snapshot) {
  bindGraphInteractions();
  const nodes = snapshot.nodes || [];
  const clients = snapshot.clients || [];
  const subscribers = snapshot.subscribers || [];

  // Vertices.
  const vertices = nodes.map((n) => ({ id: n.id, kind: "node" }));
  for (const c of clients) vertices.push({ id: `client:${c.id}`, kind: "client", ref: c });
  for (const s of subscribers) vertices.push({ id: `sub:${s.id}`, kind: "subscriber", ref: s });

  // Edges: cluster node pairs (deduped) + entity -> connected node.
  const edges = [];
  const drawnPairs = new Set();
  for (const link of snapshot.links || []) {
    const key = [link.from, link.to].slice().sort().join("|");
    if (drawnPairs.has(key)) continue;
    drawnPairs.add(key);
    const reverse = (snapshot.links || []).find((o) => o.from === link.to && o.to === link.from);
    edges.push({ a: link.from, b: link.to, kind: "node", state: worstLinkState(link.state, reverse?.state), from: link.from, to: link.to });
  }
  for (const c of clients) {
    if (c.connected_node) edges.push({ a: c.connected_node, b: `client:${c.id}`, kind: "entity", active: Boolean(c.last_op) });
  }
  for (const s of subscribers) {
    if (s.connected_node) edges.push({ a: s.connected_node, b: `sub:${s.id}`, kind: "entity", active: Boolean(s.last_event) });
  }

  graphSim.vertices = vertices;
  graphSim.edges = edges;

  // Persist / initialise positions.
  const liveIds = new Set(vertices.map((v) => v.id));
  for (const id of [...graphSim.pos.keys()]) if (!liveIds.has(id)) graphSim.pos.delete(id);
  vertices.forEach((v, index) => ensureVertexPosition(v, index, vertices.length));

  // Rebuild DOM only when the set of vertices/edges changes.
  const topoKey = vertices.map((v) => v.id).join(",") + "::" + edges.map((e) => `${e.a}>${e.b}`).join(",");
  if (topoKey !== graphSim.topoKey) {
    buildGraphDom(snapshot);
    graphSim.topoKey = topoKey;
  }

  updateGraphVisuals(snapshot);
  rebuildPackets(snapshot);
  startGraphSim();
}

function ensureVertexPosition(vertex, index, count) {
  if (graphSim.pos.has(vertex.id)) {
    graphSim.pos.get(vertex.id).kind = vertex.kind;
    return;
  }
  let x;
  let y;
  if (vertex.kind === "node") {
    const angle = -Math.PI / 2 + (index * Math.PI * 2) / Math.max(count, 1);
    x = GRAPH.center.x + Math.cos(angle) * 150;
    y = GRAPH.center.y + Math.sin(angle) * 150;
  } else {
    // Spawn next to the node it connects to, so it eases in instead of flying.
    const nodeId = vertex.ref?.connected_node;
    const anchor = nodeId ? graphSim.pos.get(nodeId) : null;
    const base = anchor || GRAPH.center;
    x = base.x + (Math.random() - 0.5) * 60;
    y = base.y + (Math.random() - 0.5) * 60 + 70;
  }
  graphSim.pos.set(vertex.id, { x, y, vx: 0, vy: 0, fixed: false, kind: vertex.kind });
}

function buildGraphDom(snapshot) {
  el.graph.replaceChildren();
  const viewport = svg("g", { class: "graph-viewport" });
  const edgeLayer = svg("g", { class: "links" });
  const entityLinkLayer = svg("g", { class: "entity-links", "aria-hidden": "true" });
  const packetLayer = svg("g", { class: "packet-layer", "aria-hidden": "true" });
  const pulseLayer = svg("g", { class: "pulse-layer", "aria-hidden": "true" });
  const vertexLayer = svg("g", { class: "vertices" });
  viewport.append(edgeLayer, entityLinkLayer, packetLayer, pulseLayer, vertexLayer);
  el.graph.append(viewport);

  const dom = { viewport, packetLayer, pulseLayer, edges: [], entityEdges: [], vertices: new Map() };

  for (const edge of graphSim.edges) {
    if (edge.kind === "node") {
      const visual = svg("line", { class: "link" });
      const hit = svg("line", { class: "link-hit" });
      hit.addEventListener("click", (event) => {
        event.stopPropagation();
        state.selectedLink = { from: edge.from, to: edge.to };
        render();
      });
      edgeLayer.append(visual, hit);
      dom.edges.push({ edge, visual, hit });
    } else {
      const line = svg("line", { class: "entity-link" });
      entityLinkLayer.append(line);
      dom.entityEdges.push({ edge, line });
    }
  }

  for (const vertex of graphSim.vertices) {
    const group = svg("g", { class: "vertex" });
    if (vertex.kind === "node") {
      group.append(svg("circle", { r: GRAPH.nodeRadius }), svg("text", { class: "v-id", y: -5 }), svg("text", { class: "v-sub", y: 17 }));
    } else {
      group.append(
        svg("circle", { r: GRAPH.entityRadius }),
        svg("text", { class: "entity-id", y: 4 }),
        svg("text", { class: "entity-sub", y: 36 }),
      );
      if (vertex.kind === "client") group.append(svg("title", {}, "Click to push (drag to move)"));
    }
    bindVertexDrag(group, vertex);
    vertexLayer.append(group);
    dom.vertices.set(vertex.id, { group, vertex });
  }

  graphSim.dom = dom;
}

function updateGraphVisuals(snapshot) {
  const nodeById = new Map((snapshot.nodes || []).map((n) => [n.id, n]));
  for (const [, item] of graphSim.dom.vertices) {
    const v = item.vertex;
    if (v.kind === "node") {
      const node = nodeById.get(v.id);
      if (!node) continue;
      const status = nodeStatus(node);
      const isNew = !state.seenNodes.has(node.id);
      state.seenNodes.add(node.id);
      item.group.setAttribute(
        "class",
        ["vertex", "node", `role-${status}`, node.crashed ? "crashed" : "", node.disabled ? "disabled" : "", isNew ? "joining" : ""].filter(Boolean).join(" "),
      );
      item.group.querySelector(".v-id").textContent = node.id;
      item.group.querySelector(".v-sub").textContent = status;
    } else {
      const ref = v.ref;
      const active = v.kind === "client" ? Boolean(ref.last_op) : Boolean(ref.last_event);
      item.group.setAttribute("class", ["vertex", "entity", v.kind, active ? "active" : "", v.kind === "client" ? "clickable" : ""].filter(Boolean).join(" "));
      item.group.querySelector(".entity-id").textContent = v.kind === "client" ? ref.id : ref.client_id || ref.id;
      item.group.querySelector(".entity-sub").textContent = ref.namespace || "";
    }
  }
  for (const { edge, visual } of graphSim.dom.edges) {
    const selected = state.selectedLink && state.selectedLink.from === edge.from && state.selectedLink.to === edge.to;
    visual.setAttribute("class", ["link", edge.state, selected ? "selected" : ""].filter(Boolean).join(" "));
  }
  for (const { edge, line } of graphSim.dom.entityEdges) {
    line.setAttribute("class", ["entity-link", edge.kind === "entity" && edge.b.startsWith("sub:") ? "subscriber" : "client", edge.active ? "active" : ""].filter(Boolean).join(" "));
  }
}

function rebuildPackets(snapshot) {
  graphSim.packets = [];
  if (!graphSim.dom) return;
  graphSim.dom.packetLayer.replaceChildren();
  const messages = (snapshot.in_flight || []).slice(0, 16);
  messages.forEach((message, index) => {
    if (!graphSim.pos.has(message.from) || !graphSim.pos.has(message.to)) return;
    const kind = packetKind(message.kind);
    const trail = svg("line", { class: `packet-trail ${kind}` });
    const dot = svg("circle", { class: `packet ${kind}`, r: 7 });
    graphSim.dom.packetLayer.append(trail, dot);
    graphSim.packets.push({ dot, trail, from: message.from, to: message.to, offset: index / Math.max(messages.length, 1) });
  });
}

function applyGraphForces() {
  const V = graphSim.vertices;
  const P = graphSim.pos;
  for (let i = 0; i < V.length; i += 1) {
    const pi = P.get(V[i].id);
    for (let j = i + 1; j < V.length; j += 1) {
      const pj = P.get(V[j].id);
      let dx = pi.x - pj.x;
      let dy = pi.y - pj.y;
      let d2 = dx * dx + dy * dy;
      if (d2 < 1) {
        dx = Math.random() - 0.5;
        dy = Math.random() - 0.5;
        d2 = 1;
      }
      const f = GRAPH.repulsion / d2;
      const d = Math.sqrt(d2);
      const fx = (dx / d) * f;
      const fy = (dy / d) * f;
      pi.vx += fx;
      pi.vy += fy;
      pj.vx -= fx;
      pj.vy -= fy;
    }
  }
  for (const e of graphSim.edges) {
    const pa = P.get(e.a);
    const pb = P.get(e.b);
    if (!pa || !pb) continue;
    let dx = pb.x - pa.x;
    let dy = pb.y - pa.y;
    const d = Math.hypot(dx, dy) || 1;
    const rest = e.kind === "entity" ? GRAPH.springEntity : GRAPH.springNode;
    const f = (d - rest) * GRAPH.spring;
    const fx = (dx / d) * f;
    const fy = (dy / d) * f;
    pa.vx += fx;
    pa.vy += fy;
    pb.vx -= fx;
    pb.vy -= fy;
  }
  let energy = 0;
  const padX = 74;
  const padY = 54;
  for (const v of V) {
    const p = P.get(v.id);
    // Pull everything gently toward the centre; entities a little less so they
    // can sit outside the cluster but still drift in-screen when there's room.
    const pull = v.kind === "node" ? GRAPH.centerPull : GRAPH.centerPull * 0.5;
    p.vx += (GRAPH.center.x - p.x) * pull;
    p.vy += (GRAPH.center.y - p.y) * pull;
    if (p.fixed) {
      p.vx = 0;
      p.vy = 0;
      continue;
    }
    p.vx = Math.max(-GRAPH.maxVelocity, Math.min(GRAPH.maxVelocity, p.vx * GRAPH.damping));
    p.vy = Math.max(-GRAPH.maxVelocity, Math.min(GRAPH.maxVelocity, p.vy * GRAPH.damping));
    p.x += p.vx;
    p.y += p.vy;
    // Keep physics-driven vertices inside the visible viewBox so clients and
    // subscribers do not drift off-screen (a held drag is exempt above).
    if (p.x < padX) {
      p.x = padX;
      p.vx = 0;
    } else if (p.x > 800 - padX) {
      p.x = 800 - padX;
      p.vx = 0;
    }
    if (p.y < padY) {
      p.y = padY;
      p.vy = 0;
    } else if (p.y > 520 - padY) {
      p.y = 520 - padY;
      p.vy = 0;
    }
    energy += p.vx * p.vx + p.vy * p.vy;
  }
  return energy;
}

function paintGraph(now) {
  const P = graphSim.pos;
  if (graphSim.dom) {
    graphSim.dom.viewport.setAttribute(
      "transform",
      `translate(${graphSim.view.tx} ${graphSim.view.ty}) scale(${graphSim.view.scale})`,
    );
    for (const [id, item] of graphSim.dom.vertices) {
      const p = P.get(id);
      if (p) item.group.setAttribute("transform", `translate(${p.x} ${p.y})`);
    }
    const setLine = (line, a, b) => {
      const pa = P.get(a);
      const pb = P.get(b);
      if (!pa || !pb) return;
      line.setAttribute("x1", pa.x);
      line.setAttribute("y1", pa.y);
      line.setAttribute("x2", pb.x);
      line.setAttribute("y2", pb.y);
    };
    for (const { edge, visual, hit } of graphSim.dom.edges) {
      setLine(visual, edge.a, edge.b);
      setLine(hit, edge.a, edge.b);
    }
    for (const { edge, line } of graphSim.dom.entityEdges) setLine(line, edge.a, edge.b);
  }
  const reduced = document.documentElement.dataset.reducedMotion === "true";
  const base = (now / 1150) % 1;
  for (const packet of graphSim.packets) {
    const pa = P.get(packet.from);
    const pb = P.get(packet.to);
    if (!pa || !pb) continue;
    const progress = reduced ? 0.5 : (base + packet.offset) % 1;
    const x = pa.x + (pb.x - pa.x) * progress;
    const y = pa.y + (pb.y - pa.y) * progress;
    packet.dot.setAttribute("cx", x);
    packet.dot.setAttribute("cy", y);
    const tail = Math.max(0, progress - 0.16);
    packet.trail.setAttribute("x1", pa.x + (pb.x - pa.x) * tail);
    packet.trail.setAttribute("y1", pa.y + (pb.y - pa.y) * tail);
    packet.trail.setAttribute("x2", x);
    packet.trail.setAttribute("y2", y);
  }
  updatePulses(now);
}

// Transient, choreographed "data flow" triggered by a client push: request travels
// client -> entry node, replicates entry node -> peers, then is delivered entry
// node(s) -> subscriber(s). Decorative overlay on its own layer (survives refresh).
function updatePulses(now) {
  if (graphSim.pulses.length === 0) {
    return;
  }
  const P = graphSim.pos;
  const remaining = [];
  for (const pulse of graphSim.pulses) {
    let alive = false;
    for (const mover of pulse.movers) {
      const t = now - pulse.t0 - mover.startMs;
      if (t < 0) {
        mover.dot.setAttribute("opacity", "0");
        alive = true;
        continue;
      }
      const progress = t / mover.durMs;
      if (progress >= 1) {
        mover.dot.setAttribute("opacity", "0");
        continue;
      }
      alive = true;
      const pa = P.get(mover.fromId);
      const pb = P.get(mover.toId);
      if (!pa || !pb) {
        continue;
      }
      mover.dot.setAttribute("opacity", "1");
      mover.dot.setAttribute("cx", pa.x + (pb.x - pa.x) * progress);
      mover.dot.setAttribute("cy", pa.y + (pb.y - pa.y) * progress);
    }
    if (alive) {
      remaining.push(pulse);
    } else {
      for (const mover of pulse.movers) mover.dot.remove();
    }
  }
  graphSim.pulses = remaining;
}

function playDataFlowPulse(clientId, namespace) {
  if (!graphSim.dom || !state.snapshot) {
    return;
  }
  if (document.documentElement.dataset.reducedMotion === "true") {
    return;
  }
  const snapshot = state.snapshot;
  const clientVertexId = `client:${clientId}`;
  const client = (snapshot.clients || []).find((c) => c.id === clientId);
  const entryNode = client?.connected_node;
  if (!entryNode || !graphSim.pos.has(clientVertexId)) {
    return;
  }
  const peers = (snapshot.nodes || [])
    .filter((n) => !n.crashed && !n.disabled && n.id !== entryNode)
    .map((n) => n.id);
  const subscribers = (snapshot.subscribers || []).filter(
    (s) => s.namespace === namespace && s.connected_node,
  );

  const movers = [];
  const add = (fromId, toId, startMs, durMs, kind) => {
    if (!graphSim.pos.has(fromId) || !graphSim.pos.has(toId)) {
      return;
    }
    const dot = svg("circle", { class: `pulse ${kind}`, r: 6, opacity: 0, cx: 0, cy: 0 });
    graphSim.dom.pulseLayer.append(dot);
    movers.push({ fromId, toId, startMs, durMs, kind, dot });
  };

  // 1) request into the cluster
  add(clientVertexId, entryNode, 0, 460, "data");
  // 2) replication across the live cluster
  peers.forEach((peer, index) => add(entryNode, peer, 340 + index * 45, 460, "data"));
  const replicationEnd = 340 + Math.max(0, peers.length - 1) * 45 + 460;
  // 3) delivery to each subscriber on the namespace
  subscribers.forEach((sub, index) =>
    add(sub.connected_node, `sub:${sub.id}`, replicationEnd + index * 70, 520, "event"),
  );

  if (movers.length === 0) {
    return;
  }
  graphSim.pulses.push({ t0: performance.now(), movers });
  startGraphSim();
}

function startGraphSim() {
  if (graphSim.raf !== null) return;
  const tick = (now) => {
    const energy = applyGraphForces();
    paintGraph(now);
    const busy =
      graphSim.drag || graphSim.packets.length > 0 || graphSim.pulses.length > 0 || energy > 0.05;
    graphSim.raf = busy ? requestAnimationFrame(tick) : null;
    if (!busy) paintGraph(now); // final settle paint
  };
  graphSim.raf = requestAnimationFrame(tick);
}

function pointerToGraph(event) {
  const ctm = el.graph.getScreenCTM();
  if (!ctm) return { x: 0, y: 0 };
  const inv = ctm.inverse();
  const gx = inv.a * event.clientX + inv.c * event.clientY + inv.e;
  const gy = inv.b * event.clientX + inv.d * event.clientY + inv.f;
  return {
    x: (gx - graphSim.view.tx) / graphSim.view.scale,
    y: (gy - graphSim.view.ty) / graphSim.view.scale,
  };
}

function bindVertexDrag(group, vertex) {
  group.addEventListener("pointerdown", (event) => {
    event.stopPropagation();
    const p = graphSim.pos.get(vertex.id);
    if (!p) return;
    graphSim.movedDuringPress = false;
    const start = { x: event.clientX, y: event.clientY };
    p.fixed = true;
    graphSim.drag = { id: vertex.id };
    startGraphSim();
    group.setPointerCapture?.(event.pointerId);

    const move = (ev) => {
      if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 4) graphSim.movedDuringPress = true;
      const g = pointerToGraph(ev);
      p.x = g.x;
      p.y = g.y;
      p.vx = 0;
      p.vy = 0;
      startGraphSim();
    };
    const up = (ev) => {
      p.fixed = false;
      graphSim.drag = null;
      group.releasePointerCapture?.(ev.pointerId);
      group.removeEventListener("pointermove", move);
      group.removeEventListener("pointerup", up);
      if (!graphSim.movedDuringPress && vertex.kind === "client") {
        void pushEventFor(vertex.ref.id, vertex.ref.namespace);
      }
      startGraphSim();
    };
    group.addEventListener("pointermove", move);
    group.addEventListener("pointerup", up);
  });
}

function bindGraphInteractions() {
  if (graphSim.interactionsBound) return;
  graphSim.interactionsBound = true;
  // Pan the whole graph by dragging the empty canvas.
  el.graph.addEventListener("pointerdown", (event) => {
    if (event.target.closest(".vertex") || event.target.closest(".link-hit")) return;
    const ctm = el.graph.getScreenCTM();
    const sx = ctm && ctm.a ? ctm.a : 1;
    const sy = ctm && ctm.d ? ctm.d : 1;
    const start = { x: event.clientX, y: event.clientY, tx: graphSim.view.tx, ty: graphSim.view.ty };
    graphSim.drag = { pan: true };
    el.graph.setPointerCapture?.(event.pointerId);
    const move = (ev) => {
      graphSim.view.tx = start.tx + (ev.clientX - start.x) / sx;
      graphSim.view.ty = start.ty + (ev.clientY - start.y) / sy;
      paintGraph(performance.now());
    };
    const up = (ev) => {
      graphSim.drag = null;
      el.graph.releasePointerCapture?.(ev.pointerId);
      el.graph.removeEventListener("pointermove", move);
      el.graph.removeEventListener("pointerup", up);
    };
    el.graph.addEventListener("pointermove", move);
    el.graph.addEventListener("pointerup", up);
  });
  // Wheel zoom around the cursor.
  el.graph.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const before = pointerToGraph(event);
      const factor = event.deltaY < 0 ? 1.1 : 1 / 1.1;
      graphSim.view.scale = Math.max(0.45, Math.min(2.4, graphSim.view.scale * factor));
      const after = pointerToGraph(event);
      graphSim.view.tx += (after.x - before.x) * graphSim.view.scale;
      graphSim.view.ty += (after.y - before.y) * graphSim.view.scale;
      paintGraph(performance.now());
    },
    { passive: false },
  );
}

function renderSelectedLink() {
  const link = selectedLinkView();
  el.selectedLink.textContent = link
    ? `${link.from} -> ${link.to} (${link.state})`
    : "none";
  for (const button of el.linkActions) {
    button.disabled = !link;
  }
}

function renderProgress(snapshot) {
  const progress = snapshot.progress;
  el.progress.replaceChildren(
    term("Step", snapshot.step),
    desc(snapshot.step),
    term("Logical time", snapshot.logical_time_millis),
    desc(`${snapshot.logical_time_millis} ms`),
    term("Committed", progress.committed_entries),
    desc(progress.committed_entries),
    term("Formation", snapshot.formation_phase || "unknown"),
    desc(snapshot.formation_phase || "unknown"),
    term("Election", snapshot.election_source || "unknown"),
    desc(snapshot.election_source || "unknown"),
    term("Convergence", progress.convergence),
    desc(progress.convergence),
    term("Rebalance", snapshot.rebalance?.phase || "idle"),
    desc(snapshot.rebalance ? `${snapshot.rebalance.moved_partitions}/${snapshot.rebalance.total_partitions}` : "idle"),
    term("Mode", snapshot.mode || "manual"),
    desc(snapshot.active_scenario || "manual"),
  );
  el.hash.textContent = `snapshot ${snapshotHash(snapshot)}`;
}

function renderNodes(snapshot) {
  const list = document.createElement("div");
  list.className = "metric-list";
  for (const node of snapshot.nodes) {
    const status = nodeStatus(node);
    const row = document.createElement("div");
    row.className = `metric node-row role-${status}`;
    const label = document.createElement("strong");
    label.textContent = `${node.id} ${status}`;
    const meta = document.createElement("span");
    meta.textContent = `term ${node.term}; votes ${node.votes_received}`;
    const buttons = document.createElement("div");
    buttons.className = "button-row";
    const crash = nodeButton(node.crashed ? "Restart" : "Crash", async () => {
      if (node.crashed) {
        await state.sim.restart_node(node.id);
      } else {
        await state.sim.crash_node(node.id);
      }
      await settleAfterIntervention();
      await refresh();
    });
    const isolate = nodeButton("Isolate", async () => {
      await state.sim.isolate_node(node.id);
      await settleAfterIntervention();
      await refresh();
    });
    const rejoin = nodeButton("Rejoin", async () => {
      await state.sim.rejoin_node(node.id);
      await settleAfterIntervention();
      await refresh();
    });
    const disable = nodeButton(node.disabled ? "Enable" : "Disable", async () => {
      if (node.disabled) {
        await state.sim.enable_node(node.id);
      } else {
        await state.sim.disable_node(node.id);
      }
      await settleAfterIntervention();
      await refresh();
    });
    buttons.append(crash, isolate, rejoin, disable);
    row.append(label, meta, buttons);
    list.append(row);
  }
  el.nodes.replaceChildren(list);
}

function nodeButton(label, handler) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.addEventListener("click", handler);
  return button;
}

function renderSignals(snapshot) {
  const list = document.createElement("div");
  list.className = "metric-list";
  const messages = (snapshot.in_flight || []).slice(0, 8);
  if (messages.length === 0) {
    const empty = document.createElement("p");
    empty.textContent = "no in-flight messages";
    list.append(empty);
  }
  for (const message of messages) {
    const row = document.createElement("div");
    row.className = "metric";
    const label = document.createElement("strong");
    label.textContent = message.kind;
    const meta = document.createElement("span");
    const key = message.key ? ` key ${message.key}` : "";
    meta.textContent = `${message.from} -> ${message.to}; ${message.remaining_millis} ms${key}`;
    row.append(label, meta);
    list.append(row);
  }
  const summarized = snapshot.over_budget?.in_flight_summarized || 0;
  if (summarized > 0) {
    const row = document.createElement("div");
    row.className = "metric";
    const label = document.createElement("strong");
    label.textContent = "summarized";
    const meta = document.createElement("span");
    meta.textContent = `${summarized} over render budget`;
    row.append(label, meta);
    list.append(row);
  }
  el.signals.replaceChildren(list);
}

function renderClients(snapshot) {
  const clients = document.createElement("div");
  clients.className = "metric-list";
  for (const client of snapshot.clients || []) {
    const row = document.createElement("div");
    row.className = "metric";
    const label = document.createElement("strong");
    label.textContent = client.id;
    const meta = document.createElement("span");
    meta.textContent = client.last_op || client.namespace;
    row.append(label, meta);
    clients.append(row);
  }
  if (!clients.childElementCount) {
    const empty = document.createElement("p");
    empty.textContent = "no manual clients";
    clients.append(empty);
  }

  const subscribers = document.createElement("div");
  subscribers.className = "metric-list";
  for (const subscriber of snapshot.subscribers || []) {
    const row = document.createElement("div");
    row.className = "metric";
    const label = document.createElement("strong");
    label.textContent = subscriber.id;
    const event = subscriber.last_event
      ? `${subscriber.last_event.kind} ${subscriber.last_event.key}`
      : "waiting";
    const meta = document.createElement("span");
    meta.textContent = `${event}; lag ${subscriber.lag}; drop ${subscriber.dropped}`;
    row.append(label, meta);
    subscribers.append(row);
  }
  if (!subscribers.childElementCount) {
    const empty = document.createElement("p");
    empty.textContent = "no subscribers";
    subscribers.append(empty);
  }
  el.clients.replaceChildren(clients);
  el.subscribers.replaceChildren(subscribers);
}

function renderConsistency(snapshot) {
  const list = document.createElement("div");
  list.className = "metric-list";
  const value = snapshot.verdict.status === "holding" ? snapshot.progress.convergence : "blocked";
  for (const level of CONSISTENCY_LEVELS) {
    const row = document.createElement("div");
    row.className = "metric";
    const label = document.createElement("strong");
    label.textContent = level;
    const status = document.createElement("span");
    status.textContent = value;
    row.append(label, status);
    list.append(row);
  }
  el.consistency.replaceChildren(list);
}

function renderKeys(snapshot) {
  const list = document.createElement("div");
  list.className = "key-list";
  const keys = snapshot.keys.slice(0, 8);
  if (keys.length === 0) {
    const empty = document.createElement("p");
    empty.textContent = "no sampled keys";
    list.append(empty);
  }
  for (const key of keys) {
    const row = document.createElement("div");
    row.className = "key-row";
    const label = document.createElement("strong");
    label.textContent = key.key;
    const value = document.createElement("span");
    value.textContent = `${key.replicas.length} replicas`;
    row.append(label, value);
    list.append(row);
  }
  el.keys.replaceChildren(list);
}

async function applyLinkAction(action) {
  const link = selectedLinkView();
  if (!link || !state.sim) {
    return;
  }
  const delay = action === "delay" ? BigInt(250) : BigInt(0);
  await state.sim.inject(action, link.from, link.to, delay);
  if (action === "delay" || action === "drop") {
    await state.sim.step();
  }
  await settleAfterIntervention();
  await refresh();
}

function togglePlay() {
  if (state.playing) {
    stopPlay();
  } else {
    startPlay();
  }
}

// After a manual intervention (add/crash/isolate/rejoin/disable/push), advance
// the modeled clock a few ticks so the cluster visibly reacts -- joiners become
// followers, a lost leader is re-elected -- instead of sitting frozen on a paused
// sim until the user presses Step. These are ordinary deterministic steps, so the
// run stays seed-reproducible. No-op while auto-playing (the interval steps already)
// and capped so a legitimately leaderless cluster cannot spin forever.
function manualClientId() {
  const value = (el.manualClient?.value || "").trim();
  return value || "client-a";
}

// Push an event for a specific client (used by the Push button and by clicking a
// client in the graph). Uses the panel key/value as the payload.
async function pushEventFor(client, namespace) {
  if (!state.sim) {
    return;
  }
  // Start the visual data-flow before the engine call so the request is seen
  // travelling into the cluster while the push commits and replicates.
  playDataFlowPulse(client, namespace);
  await state.sim.push_event(client, namespace, el.manualKey.value, el.manualValue.value);
  await settleAfterIntervention();
  await refresh();
}

async function settleAfterIntervention() {
  if (state.playing || !state.sim) {
    return;
  }
  for (let attempt = 0; attempt < 12; attempt += 1) {
    await state.sim.step();
    const snapshot = JSON.parse(await state.sim.snapshot_json());
    const nodes = snapshot.nodes || [];
    const stillJoining = nodes.some(
      (node) =>
        !node.crashed &&
        !node.disabled &&
        (node.vote_state === "joining" || node.vote_state === "catching_up"),
    );
    const hasLeader = nodes.some((node) => node.vote_state === "leader");
    if (hasLeader && !stillJoining) {
      break;
    }
  }
}

async function copyReproducer() {
  if (!state.snapshot) {
    return;
  }
  const replayScriptJson = (await state.sim?.replay_script_json?.()) ?? "";
  const command = reproducerCommand(
    state.snapshot.seed,
    state.snapshot.step,
    replayScriptJson,
    window.location.href,
  );
  try {
    await navigator.clipboard?.writeText(command);
    el.copyStatus.textContent = command;
  } catch (_error) {
    el.copyStatus.textContent = command;
  }
}

async function loadReplayScript(script) {
  if (state.engine === "wasm" && !state.SimHandle) {
    return;
  }
  stopPlay();
  state.selectedLink = null;
  state.scenario = script.scenario || "default";
  el.scenario.value = state.scenario;
  state.sim = await createSimulation(script.seed);
  await state.sim.apply_control_script_json(JSON.stringify(script));
  await refresh();
  el.seedInput.value = String(state.snapshot.seed);
}

function startPlay() {
  if (!state.sim || state.playing) {
    return;
  }
  state.playing = true;
  el.play.textContent = "Pause";
  state.timer = window.setInterval(async () => {
    if (state.tickPending) {
      return;
    }
    state.tickPending = true;
    try {
      await state.sim.step();
      await refresh();
    } finally {
      state.tickPending = false;
    }
  }, Number(el.speed.value));
}

function stopPlay() {
  state.playing = false;
  el.play.textContent = "Play";
  if (state.timer) {
    window.clearInterval(state.timer);
    state.timer = null;
  }
}

function selectedLinkView() {
  if (!state.selectedLink || !state.snapshot) {
    return null;
  }
  return state.snapshot.links.find(
    (link) => link.from === state.selectedLink.from && link.to === state.selectedLink.to,
  );
}

function isSelectedLink(link) {
  return (
    state.selectedLink &&
    state.selectedLink.from === link.from &&
    state.selectedLink.to === link.to
  );
}

// When collapsing the two directed links of a pair into one drawn line, show the
// most degraded state so a partition/delay is never hidden by a healthy reverse.
function worstLinkState(a, b) {
  const rank = { partitioned: 3, delayed: 2, up: 1 };
  const left = rank[a] || 0;
  const right = rank[b] || 0;
  return right > left ? b : a;
}

function nodeStatus(node) {
  if (node.crashed) {
    return "crashed";
  }
  if (node.disabled) {
    return "disabled";
  }
  return String(node.vote_state || node.role || "unknown");
}

function packetKind(kind) {
  const normalized = String(kind || "message")
    .trim()
    .toLowerCase()
    .replace(/_/g, "-")
    .replace(/[^a-z0-9-]/g, "-");
  if (normalized.includes("heartbeat")) {
    return "heartbeat";
  }
  if (normalized.includes("vote")) {
    return `vote ${normalized}`;
  }
  if (normalized.includes("upsert") || normalized.includes("data")) {
    return `data ${normalized}`;
  }
  if (normalized.includes("invalid") || normalized.includes("event")) {
    return `event ${normalized}`;
  }
  return normalized || "message";
}

function svg(name, attrs = {}, text = null) {
  const node = document.createElementNS(SVG_NS, name);
  for (const [key, value] of Object.entries(attrs)) {
    node.setAttribute(key, String(value));
  }
  if (text !== null) {
    node.textContent = text;
  }
  return node;
}

function term(text) {
  const node = document.createElement("dt");
  node.textContent = text;
  return node;
}

function desc(text) {
  const node = document.createElement("dd");
  node.textContent = String(text);
  return node;
}

function readSeed() {
  const parsed = Number.parseInt(el.seedInput.value, 10);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : 80;
}

function populateScenarios(selected) {
  el.scenario.replaceChildren();
  for (const scenario of SCENARIOS) {
    const option = document.createElement("option");
    option.value = scenario.name;
    option.textContent = scenario.title;
    option.title = scenario.summary;
    option.selected = scenario.name === selected;
    el.scenario.append(option);
  }
}

function bindPreferenceState() {
  bindMediaFlag("reducedMotion", "(prefers-reduced-motion: reduce)");
  bindMediaFlag("reducedTransparency", "(prefers-reduced-transparency: reduce)");
}

function bindMediaFlag(name, query) {
  if (typeof window.matchMedia !== "function") {
    document.documentElement.dataset[name] = "false";
    return;
  }
  const media = window.matchMedia(query);
  const apply = () => {
    document.documentElement.dataset[name] = media.matches ? "true" : "false";
  };
  apply();
  if (typeof media.addEventListener === "function") {
    media.addEventListener("change", apply);
  } else if (typeof media.addListener === "function") {
    media.addListener(apply);
  }
}

async function createSimulation(seed) {
  resetGraph();
  state.seenNodes = new Set();
  if (state.engine === "server") {
    const sim = new ServerSimSession(state.apiBase);
    await sim.reset(seed);
    return sim;
  }
  return new WasmSimSession(new state.SimHandle(BigInt(seed)));
}

class WasmSimSession {
  constructor(handle) {
    this.handle = handle;
  }

  async set_workload_enabled(enabled) {
    this.handle.set_workload_enabled(enabled);
  }

  async set_mode(mode) {
    return this.handle.set_mode(mode);
  }

  async run(steps) {
    this.handle.run(toBigInt(steps));
  }

  async step() {
    this.handle.step();
  }

  async apply_scenario(name) {
    this.handle.apply_scenario(name);
  }

  async apply_control_script_json(scriptJson) {
    return this.handle.apply_control_script_json(scriptJson);
  }

  async subscribe(client, namespace) {
    this.handle.subscribe(client, namespace);
  }

  async push_event(client, namespace, key, value) {
    return this.handle.push_event(client, namespace, key, value);
  }

  async snapshot_json() {
    return this.handle.snapshot_json();
  }

  async replay_script_json() {
    return this.handle.replay_script_json();
  }

  async restart_node(node) {
    return this.handle.restart_node(node);
  }

  async isolate_node(node) {
    return this.handle.isolate_node(node);
  }

  async rejoin_node(node) {
    return this.handle.rejoin_node(node);
  }

  async disable_node(node) {
    return this.handle.disable_node(node);
  }

  async enable_node(node) {
    return this.handle.enable_node(node);
  }

  async add_node() {
    return this.handle.add_node();
  }

  async crash_node(node) {
    return this.handle.crash_node(node);
  }

  async inject(action, from, to, delay) {
    return this.handle.inject(action, from, to, toBigInt(delay));
  }
}

class ServerSimSession {
  constructor(apiBase) {
    this.apiBase = apiBase;
    this.snapshot = null;
  }

  async reset(seed) {
    this.snapshot = await this.post("/sim/new", { seed });
  }

  async set_workload_enabled(enabled) {
    this.snapshot = await this.post("/sim/inject", { action: "workload", enabled });
  }

  async set_mode(mode) {
    this.snapshot = await this.post("/sim/inject", { action: "mode_change", mode });
  }

  async run(steps) {
    this.snapshot = await this.post("/sim/step", { steps: toNumber(steps) });
  }

  async step() {
    await this.run(1);
  }

  async apply_scenario(name) {
    this.snapshot = await this.post("/sim/new", { scenario: name });
  }

  async apply_control_script_json(scriptJson) {
    this.snapshot = await this.post("/sim/control", JSON.parse(scriptJson));
  }

  async subscribe(client, ns) {
    this.snapshot = await this.post("/sim/inject", { action: "subscribe", client, ns });
  }

  async push_event(client, ns, key, value) {
    this.snapshot = await this.post("/sim/inject", {
      action: "push_event",
      client,
      ns,
      key,
      value,
    });
  }

  async snapshot_json() {
    if (!this.snapshot) {
      this.snapshot = await this.get("/sim/snapshot");
    }
    return JSON.stringify(this.snapshot);
  }

  async replay_script_json() {
    return JSON.stringify({
      version: 1,
      seed: this.snapshot?.seed ?? 0,
      mode: this.snapshot?.mode ?? "manual",
      scenario: this.snapshot?.active_scenario ?? null,
      actions: [],
    });
  }

  async restart_node(node) {
    this.snapshot = await this.post("/sim/inject", { action: "restart", node });
    return true;
  }

  async isolate_node(node) {
    this.snapshot = await this.post("/sim/inject", { action: "isolate", node });
    return true;
  }

  async rejoin_node(node) {
    this.snapshot = await this.post("/sim/inject", { action: "rejoin", node });
    return true;
  }

  async disable_node(node) {
    this.snapshot = await this.post("/sim/inject", { action: "disable", node });
    return true;
  }

  async enable_node(node) {
    this.snapshot = await this.post("/sim/inject", { action: "enable", node });
    return true;
  }

  async add_node() {
    this.snapshot = await this.post("/sim/inject", { action: "add_node" });
    return true;
  }

  async crash_node(node) {
    this.snapshot = await this.post("/sim/inject", { action: "crash", node });
    return true;
  }

  async inject(action, from, to, delay) {
    const body = { action, from, to };
    if (action === "delay") {
      body.millis = toNumber(delay);
    }
    this.snapshot = await this.post("/sim/inject", body);
    return true;
  }

  async get(path) {
    return this.read(await fetch(`${this.apiBase}${path}`));
  }

  async post(path, body) {
    return this.read(
      await fetch(`${this.apiBase}${path}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      }),
    );
  }

  async read(response) {
    const body = await response.text();
    if (!response.ok) {
      throw new Error(`server simulator ${response.status}: ${body}`);
    }
    return JSON.parse(body);
  }
}

function toBigInt(value) {
  return typeof value === "bigint" ? value : BigInt(value);
}

function toNumber(value) {
  const number = Number(value);
  return Number.isSafeInteger(number) && number >= 0 ? number : 0;
}

function showEngineError(error) {
  el.verdict.className = "verdict warn";
  el.verdict.textContent =
    state.engine === "server" ? "server simulator unavailable" : "wasm package unavailable";
  el.banner.textContent = String(error?.message || error);
  for (const button of [el.step, el.play, el.copy, el.loadScenario, ...el.linkActions]) {
    button.disabled = true;
  }
}

boot();
