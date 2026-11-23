import {
  MAX_SEED,
  parseSeed,
  readInitialState,
  reproducerCommand,
  snapshotHash,
  writeUrlState,
} from "./share.js";
import { SCENARIOS } from "./scenarios.js";
import {
  keyAgreement,
  leaderOf,
  scriptAtStep,
  scriptFileName,
  shortVersion,
  timelineEvents,
  verdictView,
  voprScriptCommand,
} from "./insight.js";

// Upper bound for the "run to ..." buttons, so a condition that never holds
// (for example, no quorum) stops loudly instead of spinning.
const RUN_UNTIL_LIMIT = 500;
const MAX_TIMELINE_EVENTS = 40;

const SVG_NS = "http://www.w3.org/2000/svg";
// Browsers rate-limit history.replaceState (Safari: ~100 calls / 30 s), so the
// share URL is rewritten at most this often while auto-playing.
const URL_WRITE_INTERVAL_MS = 1000;

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
  // Every engine call (play ticks and user actions) runs through this chain so
  // calls never interleave and every failure is surfaced.
  queue: Promise.resolve(),
  urlTimer: null,
  lastUrlWrite: Number.NEGATIVE_INFINITY,
  nodeRows: new Map(),
  // Timeline of the current run: the furthest recorded script, the step the
  // run starts at (a preset's own setup cannot be rewound), and whether the
  // view is currently rewound inside that recording.
  timeline: { script: null, base: 0, furthest: 0, rewound: false },
  // Per-step observations made by this page (leader, verdict) for the event list.
  observations: new Map(),
  tour: { scenario: "default", index: 0, crashed: null },
};

const el = {
  banner: document.querySelector("#engine-banner"),
  verdict: document.querySelector("#verdict"),
  notice: document.querySelector("#action-notice"),
  seedError: document.querySelector("#seed-error"),
  downloadScript: document.querySelector("#download-script"),
  scriptStatus: document.querySelector("#script-status"),
  untilButtons: Array.from(document.querySelectorAll("[data-until]")),
  timelineInput: document.querySelector("#timeline-input"),
  timelineLabel: document.querySelector("#timeline-label"),
  timelineHelp: document.querySelector("#timeline-help"),
  events: document.querySelector("#events-panel"),
  verdictPanel: document.querySelector("#verdict-panel"),
  guideTitle: document.querySelector("#guide-title"),
  guideText: document.querySelector("#guide-text"),
  guideBack: document.querySelector("#guide-back"),
  guideDo: document.querySelector("#guide-do"),
  guideNext: document.querySelector("#guide-next"),
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
  const initial = readInitialState(window.location.search, window.location.origin);
  state.engine = initial.engine;
  state.apiBase = initial.apiBase;
  state.scenario = initial.scenario;
  el.seedInput.value = String(initial.seed);
  populateScenarios(initial.scenario);
  bindEvents();
  if (initial.warnings.length > 0) {
    showNotice(initial.warnings.join(" "), "warn");
  }
  if (state.engine === "server" && initial.apiError) {
    showEngineError(new Error(initial.apiError));
    return;
  }
  await enqueue(async () => {
    try {
      if (state.engine === "wasm") {
        const wasm = await import("./pkg/hydracache_sim_wasm.js");
        if (typeof wasm.default === "function") {
          await wasm.default();
        }
        state.SimHandle = wasm.SimHandle;
      }
    } catch (error) {
      showEngineError(error);
      return;
    }
    try {
      if (initial.script) {
        await loadReplayScript(initial.script);
      } else if (initial.scenario === "default") {
        await resetSimulation(initial.steps);
      } else {
        await loadScenario(initial.scenario, initial.steps);
      }
    } catch (error) {
      if (state.engine === "server" && !state.sim) {
        showEngineError(error);
        return;
      }
      // The engine works but the shared run could not be rebuilt: say so and
      // keep the controls usable (Reset starts a fresh seeded run).
      showNotice(`Shared run could not be loaded: ${errorMessage(error)}`);
    }
  });
}

function bindEvents() {
  el.reset.addEventListener("click", () => void runAction(() => resetSimulation()));
  el.loadScenario.addEventListener(
    "click",
    () => void runAction(() => loadScenario(el.scenario.value)),
  );
  el.mode.addEventListener("change", () =>
    void runAction(async () => {
      await state.sim?.set_mode(el.mode.value);
      await refresh();
    }),
  );
  el.step.addEventListener("click", () =>
    void runAction(async () => {
      await state.sim?.step();
      await refresh();
    }),
  );
  el.play.addEventListener("click", togglePlay);
  el.copy.addEventListener("click", () => void runAction(copyReproducer, { keepTimeline: true }));
  el.downloadScript.addEventListener("click", () => void runAction(downloadScript, { keepTimeline: true }));
  for (const button of el.untilButtons) {
    button.addEventListener("click", () => void runAction(() => runUntil(button.dataset.until)));
  }
  el.timelineInput.addEventListener("change", () =>
    void runAction(() => rewindTo(Number(el.timelineInput.value)), { keepTimeline: true }),
  );
  el.guideBack.addEventListener("click", () => moveTour(-1));
  el.guideNext.addEventListener("click", () => moveTour(1));
  el.guideDo.addEventListener("click", () => void runAction(runTourStep));
  el.seedInput.addEventListener("input", () => validateSeedInput());
  el.subscribe.addEventListener("click", () =>
    void runAction(async () => {
      await state.sim?.subscribe(manualClientId(), el.manualNs.value);
      await settleAfterIntervention();
      await refresh();
    }),
  );
  el.pushEvent.addEventListener("click", () =>
    void runAction(() => pushEventFor(manualClientId(), el.manualNs.value)),
  );
  el.addNode.addEventListener("click", () =>
    void runAction(async () => {
      await state.sim?.add_node();
      await settleAfterIntervention();
      await refresh();
    }),
  );
  el.workload.addEventListener("change", () =>
    void runAction(async () => {
      await state.sim?.set_workload_enabled(el.workload.checked);
      await refresh();
    }),
  );
  el.speed.addEventListener("input", () => {
    if (state.playing) {
      stopPlay();
      startPlay();
    }
  });
  for (const button of el.linkActions) {
    button.addEventListener(
      "click",
      () => void runAction(() => applyLinkAction(button.dataset.action)),
    );
  }
}

async function resetSimulation(steps = 0) {
  if (state.engine === "wasm" && !state.SimHandle) {
    return;
  }
  const seed = readSeed();
  stopPlay();
  state.scenario = "default";
  el.scenario.value = state.scenario;
  state.selectedLink = null;
  state.sim = await createSimulation(seed);
  startTimeline("default", 0);
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
  startTimeline(state.scenario, scenarioBaseStep(state.scenario));
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
  render();
  if (state.playing) {
    scheduleUrlWrite();
  } else {
    await writeShareUrl();
  }
}

// While playing, coalesce URL rewrites to one per URL_WRITE_INTERVAL_MS; the
// trailing write goes through the engine queue so it never interleaves with a
// step. Rendering never waits on (or is skipped by) a URL write.
function scheduleUrlWrite() {
  if (state.urlTimer !== null) {
    return;
  }
  const wait = Math.max(0, state.lastUrlWrite + URL_WRITE_INTERVAL_MS - performance.now());
  state.urlTimer = window.setTimeout(() => {
    state.urlTimer = null;
    void enqueue(writeShareUrl);
  }, wait);
}

async function writeShareUrl() {
  if (state.urlTimer !== null) {
    window.clearTimeout(state.urlTimer);
    state.urlTimer = null;
  }
  if (!state.sim || !state.snapshot) {
    return;
  }
  state.lastUrlWrite = performance.now();
  try {
    const replayScriptJson = (await state.sim.replay_script_json?.()) ?? "";
    if (syncTimelineScript(replayScriptJson)) {
      renderTimeline(state.snapshot);
    }
    writeUrlState(
      window.history,
      state.snapshot,
      state.scenario,
      state.engine,
      state.apiBase,
      replayScriptJson,
    );
  } catch (error) {
    showNotice(`Share URL was not updated: ${errorMessage(error)}`, "warn");
  }
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
  el.workload.checked = Boolean(snapshot.workload_enabled);
  recordObservation(snapshot);
  renderVerdict(snapshot);
  renderVerdictPanel(snapshot);
  renderTimeline(snapshot);
  renderGuide();
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

// The header verdict always names a reproducer for *this* run: the plain
// seed command only when no intervention or scenario shaped it.
function renderVerdict(snapshot) {
  const view = verdictView(snapshot);
  el.verdict.className = `verdict ${view.kind}`;
  el.verdict.textContent = view.text;
}

// What the verdict is made of: the engine's own list of checked invariants,
// the violation detail, and what is deliberately not part of the live check.
function renderVerdictPanel(snapshot) {
  const violated = snapshot.verdict.status === "holding" ? null : snapshot.verdict.invariant;
  const summary = document.createElement("p");
  summary.className = "model-note";
  if (violated) {
    summary.textContent = `${violated} violated at step ${snapshot.step}: ${snapshot.verdict.detail}`;
  } else if (snapshot.first_violation_step !== null && snapshot.first_violation_step !== undefined) {
    summary.textContent = `Holding now. A violation was first reported at step ${snapshot.first_violation_step}.`;
  } else {
    summary.textContent = `No invariant has been violated in this run (through step ${snapshot.step}).`;
  }
  const list = document.createElement("div");
  list.className = "metric-list";
  for (const name of snapshot.checked_invariants || []) {
    const row = document.createElement("div");
    row.className = `metric invariant-row${name === violated ? " violated" : ""}`;
    const label = document.createElement("strong");
    label.textContent = name;
    const status = document.createElement("span");
    status.textContent = name === violated ? "violated" : "holding";
    row.append(label, status);
    list.append(row);
  }
  const scope = document.createElement("p");
  scope.className = "model-note";
  scope.textContent =
    "Replica-snapshot invariants (consensus-prefix, durability, tombstone-resurrection, convergence) run in the DST test suites, not in this live verdict.";
  el.verdictPanel.replaceChildren(summary, list, scope);
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
    capturePointer(group, event.pointerId);

    const move = (ev) => {
      if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 4) graphSim.movedDuringPress = true;
      const g = pointerToGraph(ev);
      p.x = g.x;
      p.y = g.y;
      p.vx = 0;
      p.vy = 0;
      startGraphSim();
    };
    // pointerup ends a press (a still press on a client pushes); pointercancel
    // and lost capture (e.g. a touch turned into a scroll) only release the drag.
    let finished = false;
    const finish = (ev, completed) => {
      if (finished) return;
      finished = true;
      p.fixed = false;
      graphSim.drag = null;
      if (group.hasPointerCapture?.(ev.pointerId)) group.releasePointerCapture(ev.pointerId);
      group.removeEventListener("pointermove", move);
      group.removeEventListener("pointerup", up);
      group.removeEventListener("pointercancel", cancel);
      group.removeEventListener("lostpointercapture", cancel);
      if (completed && !graphSim.movedDuringPress && vertex.kind === "client") {
        void runAction(() => pushEventFor(vertex.ref.id, vertex.ref.namespace));
      }
      startGraphSim();
    };
    const up = (ev) => finish(ev, true);
    const cancel = (ev) => finish(ev, false);
    group.addEventListener("pointermove", move);
    group.addEventListener("pointerup", up);
    group.addEventListener("pointercancel", cancel);
    group.addEventListener("lostpointercapture", cancel);
  });
}

// Capture can fail for a pointer the browser no longer tracks; dragging still
// works through the element's own listeners, so this is not an error.
function capturePointer(target, pointerId) {
  try {
    target.setPointerCapture?.(pointerId);
  } catch (_error) {
    // Pointer already released.
  }
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
    capturePointer(el.graph, event.pointerId);
    const move = (ev) => {
      graphSim.view.tx = start.tx + (ev.clientX - start.x) / sx;
      graphSim.view.ty = start.ty + (ev.clientY - start.y) / sy;
      paintGraph(performance.now());
    };
    let finished = false;
    const up = (ev) => {
      if (finished) return;
      finished = true;
      graphSim.drag = null;
      if (el.graph.hasPointerCapture?.(ev.pointerId)) el.graph.releasePointerCapture(ev.pointerId);
      el.graph.removeEventListener("pointermove", move);
      el.graph.removeEventListener("pointerup", up);
      el.graph.removeEventListener("pointercancel", up);
      el.graph.removeEventListener("lostpointercapture", up);
    };
    el.graph.addEventListener("pointermove", move);
    el.graph.addEventListener("pointerup", up);
    el.graph.addEventListener("pointercancel", up);
    el.graph.addEventListener("lostpointercapture", up);
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
    term("Replicas agree", keyAgreement(snapshot).agree ? "yes" : "no"),
    desc(keyAgreement(snapshot).agree ? "yes" : "no"),
    term("Rebalance", snapshot.rebalance?.phase || "idle"),
    desc(snapshot.rebalance ? `${snapshot.rebalance.moved_partitions}/${snapshot.rebalance.total_partitions}` : "idle"),
    term("Mode", snapshot.mode || "manual"),
    desc(snapshot.active_scenario || "manual"),
  );
  el.hash.textContent = `snapshot ${snapshotHash(snapshot)}`;
}

// Node rows are keyed by id and updated in place, so a button the user is
// pressing is never replaced by a re-render (auto-play re-renders every tick).
// Handlers read the node's state at click time, not at render time.
function renderNodes(snapshot) {
  let list = el.nodes.querySelector(":scope > .metric-list");
  if (!list) {
    list = document.createElement("div");
    list.className = "metric-list";
    el.nodes.replaceChildren(list);
    state.nodeRows.clear();
  }
  const live = new Set();
  snapshot.nodes.forEach((node, index) => {
    live.add(node.id);
    let view = state.nodeRows.get(node.id);
    if (!view) {
      view = createNodeRow(node.id);
      state.nodeRows.set(node.id, view);
    }
    const status = nodeStatus(node);
    view.row.className = `metric node-row role-${status}`;
    view.label.textContent = `${node.id} ${status}`;
    view.meta.textContent = `term ${node.term}; votes ${node.votes_received}`;
    view.crash.textContent = node.crashed ? "Restart" : "Crash";
    view.disable.textContent = node.disabled ? "Enable" : "Disable";
    if (list.children[index] !== view.row) {
      list.insertBefore(view.row, list.children[index] ?? null);
    }
  });
  for (const [id, view] of state.nodeRows) {
    if (!live.has(id)) {
      view.row.remove();
      state.nodeRows.delete(id);
    }
  }
}

function createNodeRow(id) {
  const row = document.createElement("div");
  row.dataset.node = id;
  const label = document.createElement("strong");
  const meta = document.createElement("span");
  const buttons = document.createElement("div");
  buttons.className = "button-row";
  const nodeAction = (handler) => () =>
    void runAction(async () => {
      const node = (state.snapshot?.nodes || []).find((candidate) => candidate.id === id);
      if (!node) {
        throw new Error(`node ${id} is no longer in the cluster`);
      }
      await handler(node);
      await settleAfterIntervention();
      await refresh();
    });
  const crash = nodeButton("Crash", nodeAction(async (node) => {
    if (node.crashed) {
      await state.sim.restart_node(node.id);
    } else {
      await state.sim.crash_node(node.id);
    }
  }));
  const isolate = nodeButton("Isolate", nodeAction((node) => state.sim.isolate_node(node.id)));
  const rejoin = nodeButton("Rejoin", nodeAction((node) => state.sim.rejoin_node(node.id)));
  const disable = nodeButton("Disable", nodeAction(async (node) => {
    if (node.disabled) {
      await state.sim.enable_node(node.id);
    } else {
      await state.sim.disable_node(node.id);
    }
  }));
  buttons.append(crash, isolate, rejoin, disable);
  row.append(label, meta, buttons);
  return { row, label, meta, crash, isolate, rejoin, disable };
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

// The simulator is one fully replicated Raft group: per-request consistency
// levels (ONE ... ALL) are not modeled, so none are shown. Only engine-backed
// values appear here.
function renderConsistency(snapshot) {
  const note = document.createElement("p");
  note.className = "model-note";
  note.textContent =
    "Per-request consistency levels (ONE, QUORUM, ALL...) are not simulated: every committed value is replicated to every node.";
  const list = document.createElement("div");
  list.className = "metric-list";
  const agreement = keyAgreement(snapshot);
  const differing = agreement.keys.filter((key) => !key.agree).length;
  for (const [label, value] of [
    ["live replicas agree on sampled keys", agreement.agree ? "yes" : `no (${differing} key(s) differ)`],
    ["committed entries", snapshot.progress.committed_entries],
  ]) {
    const row = document.createElement("div");
    row.className = "metric";
    const term = document.createElement("strong");
    term.textContent = label;
    const status = document.createElement("span");
    status.textContent = String(value);
    row.append(term, status);
    list.append(row);
  }
  el.consistency.replaceChildren(note, list);
}

// Per-node stored checksum for each sampled key (snapshot.keys), with the
// live replicas that differ from the majority highlighted.
function renderKeys(snapshot) {
  const list = document.createElement("div");
  list.className = "key-list";
  const agreement = keyAgreement(snapshot);
  const keys = agreement.keys.slice(0, 8);
  if (keys.length === 0) {
    const empty = document.createElement("p");
    empty.textContent = "no sampled keys";
    list.append(empty);
  }
  for (const key of keys) {
    const row = document.createElement("div");
    row.className = `key-row${key.agree ? "" : " diverged"}`;
    const head = document.createElement("div");
    head.className = "key-head";
    const label = document.createElement("strong");
    label.textContent = key.key;
    const status = document.createElement("span");
    status.textContent = key.agree ? "agree" : "replicas differ";
    head.append(label, status);
    const cells = document.createElement("div");
    cells.className = "replica-cells";
    for (const cell of key.cells) {
      const chip = document.createElement("span");
      chip.className = ["replica", cell.diverged ? "diverged" : "", cell.live ? "" : "down"]
        .filter(Boolean)
        .join(" ");
      chip.textContent = `${cell.node} ${shortVersion(cell.version)}`;
      chip.title = cell.version === null ? `${cell.node}: no replica` : `${cell.node}: checksum ${cell.version}`;
      cells.append(chip);
    }
    row.append(head, cells);
    list.append(row);
  }
  el.keys.replaceChildren(list);
}

// ---------------------------------------------------------------------------
// Timeline: every earlier step of the run is rebuilt by replaying its script
// prefix (interventions up to that step, resumed exactly there).
// ---------------------------------------------------------------------------
function startTimeline(scenario, base) {
  state.timeline = { script: null, base, furthest: base, rewound: false };
  state.observations = new Map();
  state.tour = { scenario, index: 0, crashed: null };
}

function scenarioBaseStep(name) {
  return SCENARIOS.find((scenario) => scenario.name === name)?.baseStep ?? 0;
}

// Returns true when the recorded script changed, so the event list re-renders.
function syncTimelineScript(replayScriptJson) {
  if (state.timeline.rewound || !replayScriptJson) {
    return false;
  }
  const changed = JSON.stringify(state.timeline.script) !== replayScriptJson;
  state.timeline.script = JSON.parse(replayScriptJson);
  state.timeline.furthest = state.snapshot.step;
  return changed;
}

// Any action other than scrubbing continues from the visible step: a rewound
// recording is forked there and its later observations are dropped.
function forkTimeline() {
  if (!state.timeline.rewound || !state.snapshot) {
    return;
  }
  for (const step of [...state.observations.keys()]) {
    if (step > state.snapshot.step) {
      state.observations.delete(step);
    }
  }
  state.timeline.rewound = false;
  state.timeline.furthest = state.snapshot.step;
}

function recordObservation(snapshot) {
  state.observations.set(snapshot.step, {
    leader: leaderOf(snapshot),
    verdict: snapshot.verdict.status,
    invariant: snapshot.verdict.invariant ?? null,
  });
  if (!state.timeline.rewound) {
    state.timeline.furthest = Math.max(state.timeline.base, snapshot.step);
  }
}

async function rewindTo(step) {
  if (!state.sim || !state.snapshot) {
    return;
  }
  stopPlay();
  if (!state.timeline.rewound) {
    state.timeline.script = JSON.parse(await state.sim.replay_script_json());
    state.timeline.furthest = state.snapshot.step;
  }
  const { base, furthest, script } = state.timeline;
  const target = Math.min(Math.max(step, base), furthest);
  if (target === state.snapshot.step) {
    await refresh();
    return;
  }
  await state.sim.apply_control_script_json(JSON.stringify(scriptAtStep(script, target)));
  state.timeline.rewound = target < furthest;
  await refresh();
}

function renderTimeline(snapshot) {
  const { base, furthest, rewound, script } = state.timeline;
  const max = Math.max(furthest, snapshot.step);
  el.timelineInput.min = String(base);
  el.timelineInput.max = String(max);
  el.timelineInput.value = String(snapshot.step);
  el.timelineInput.disabled = max <= base;
  el.timelineLabel.textContent = `step ${snapshot.step} of ${max}`;
  el.timelineHelp.textContent = rewound
    ? `Rewound to step ${snapshot.step} of a recording that reaches step ${max}. Drag forward to replay it; Step, Play or any action continues from here and drops the later recording.`
    : "Drag to rebuild any earlier step of this run from its seed and recorded actions.";
  const events = timelineEvents(script, state.observations)
    .filter((event) => event.step >= base && event.step <= max)
    .slice(-MAX_TIMELINE_EVENTS);
  const items = events.map((event) => {
    const item = document.createElement("li");
    item.className = `event ${event.kind}${event.step > snapshot.step ? " future" : ""}`;
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = `step ${event.step} · ${event.text}`;
    button.addEventListener("click", () =>
      void runAction(() => rewindTo(event.step), { keepTimeline: true }),
    );
    item.append(button);
    return item;
  });
  if (items.length === 0) {
    const empty = document.createElement("li");
    empty.className = "event empty";
    empty.textContent = "no events yet";
    items.push(empty);
  }
  el.events.replaceChildren(...items);
}

// ---------------------------------------------------------------------------
// Run until a condition holds, with a hard step limit that fails loud.
// ---------------------------------------------------------------------------
const UNTIL_CONDITIONS = {
  leader: { label: "leader", test: (snapshot) => leaderOf(snapshot) !== null },
  agree: { label: "replica agreement", test: (snapshot) => keyAgreement(snapshot).agree },
  violation: { label: "violation", test: (snapshot) => snapshot.verdict.status !== "holding" },
};

async function runUntil(name) {
  const condition = UNTIL_CONDITIONS[name];
  if (!condition || !state.sim) {
    throw new Error(`unknown run condition '${name}'`);
  }
  stopPlay();
  let snapshot = JSON.parse(await state.sim.snapshot_json());
  let steps = 0;
  while (!condition.test(snapshot) && steps < RUN_UNTIL_LIMIT) {
    await state.sim.step();
    steps += 1;
    snapshot = JSON.parse(await state.sim.snapshot_json());
  }
  await refresh();
  if (!condition.test(snapshot)) {
    throw new Error(
      `no ${condition.label} within ${RUN_UNTIL_LIMIT} steps (stopped at step ${snapshot.step})`,
    );
  }
}

// ---------------------------------------------------------------------------
// Download the replay script and say how to run it in a terminal.
// ---------------------------------------------------------------------------
async function downloadScript() {
  if (!state.sim || !state.snapshot) {
    return;
  }
  const script = JSON.parse(await state.sim.replay_script_json());
  script.target_step = state.snapshot.step;
  const fileName = scriptFileName(state.snapshot);
  const blob = new Blob([`${JSON.stringify(script, null, 2)}\n`], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.append(link);
  link.click();
  link.remove();
  // Revoking immediately can cancel a download the browser has not started yet.
  window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  el.scriptStatus.textContent = `Saved ${fileName}. Replay it in a terminal: ${voprScriptCommand(fileName)}`;
}

// ---------------------------------------------------------------------------
// Guided tours (scenarios.js): narration plus optional engine actions.
// ---------------------------------------------------------------------------
function currentTour() {
  return SCENARIOS.find((scenario) => scenario.name === state.tour.scenario)?.tour ?? [];
}

function renderGuide() {
  const tour = currentTour();
  const scenario = SCENARIOS.find((candidate) => candidate.name === state.tour.scenario);
  const index = Math.min(state.tour.index, Math.max(tour.length - 1, 0));
  const step = tour[index];
  el.guideTitle.textContent = tour.length
    ? `Guide · ${scenario.title} · ${index + 1}/${tour.length}`
    : "Guide";
  el.guideText.textContent = step?.text ?? "No guide for this run.";
  el.guideBack.disabled = index === 0;
  el.guideNext.disabled = index >= tour.length - 1;
  el.guideDo.disabled = !step?.actions?.length;
}

function moveTour(delta) {
  const tour = currentTour();
  state.tour.index = Math.min(Math.max(state.tour.index + delta, 0), Math.max(tour.length - 1, 0));
  renderGuide();
}

function tourNode(node) {
  if (node === "$leader") {
    const leader = leaderOf(state.snapshot);
    if (!leader) {
      throw new Error("there is no leader right now; run until one is elected first");
    }
    return leader;
  }
  if (node === "$crashed") {
    if (!state.tour.crashed) {
      throw new Error("this guide has not crashed a node yet");
    }
    return state.tour.crashed;
  }
  return node;
}

async function runTourStep() {
  const tour = currentTour();
  const step = tour[state.tour.index];
  for (const action of step?.actions ?? []) {
    switch (action.type) {
      case "until":
        await runUntil(action.condition);
        break;
      case "run":
        await state.sim.run(action.steps);
        break;
      case "crash": {
        const node = tourNode(action.node);
        await state.sim.crash_node(node);
        state.tour.crashed = node;
        await settleAfterIntervention();
        break;
      }
      case "restart":
        await state.sim.restart_node(tourNode(action.node));
        await settleAfterIntervention();
        break;
      case "partition":
      case "heal":
        await state.sim.inject(action.type, action.from, action.to, BigInt(0));
        await settleAfterIntervention();
        break;
      case "workload":
        await state.sim.set_workload_enabled(action.enabled);
        break;
      case "subscribe":
        await state.sim.subscribe(action.client, action.ns);
        await settleAfterIntervention();
        break;
      case "push":
        playDataFlowPulse(action.client, action.ns);
        await state.sim.push_event(action.client, action.ns, action.key, action.value);
        await settleAfterIntervention();
        break;
      default:
        throw new Error(`unknown guide action '${action.type}'`);
    }
    await refresh();
  }
  if (state.tour.index < tour.length - 1) {
    state.tour.index += 1;
  }
  renderGuide();
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
  startTimeline(state.scenario, scenarioBaseStep(state.scenario));
  await state.sim.apply_control_script_json(JSON.stringify(script));
  await refresh();
  el.seedInput.value = String(state.snapshot.seed);
}

function startPlay() {
  if (!state.sim || state.playing) {
    return;
  }
  forkTimeline();
  state.playing = true;
  el.play.textContent = "Pause";
  state.timer = window.setInterval(() => {
    if (state.tickPending) {
      return;
    }
    state.tickPending = true;
    void enqueue(async () => {
      try {
        if (!state.playing) {
          return;
        }
        await state.sim.step();
        await refresh();
      } catch (error) {
        stopPlay();
        showNotice(`Auto-play stopped: ${errorMessage(error)}`);
      } finally {
        state.tickPending = false;
      }
    });
  }, Number(el.speed.value));
}

function stopPlay() {
  state.playing = false;
  el.play.textContent = "Play";
  if (state.timer) {
    window.clearInterval(state.timer);
    state.timer = null;
  }
  // Flush a throttled URL write so the paused page links to what it shows.
  if (state.urlTimer !== null) {
    window.clearTimeout(state.urlTimer);
    state.urlTimer = null;
    void enqueue(writeShareUrl);
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

// An invalid seed fails loud instead of silently running a different seed.
function readSeed() {
  const seed = validateSeedInput();
  if (seed === null) {
    throw new Error(el.seedError.textContent);
  }
  return seed;
}

function validateSeedInput() {
  const seed = parseSeed(el.seedInput.value);
  const invalid = seed === null;
  el.seedInput.setAttribute("aria-invalid", invalid ? "true" : "false");
  el.seedError.hidden = !invalid;
  el.seedError.textContent = invalid ? `Seed must be a whole number from 0 to ${MAX_SEED}.` : "";
  return seed;
}

// ---------------------------------------------------------------------------
// Engine call queue and user-visible failures.
// ---------------------------------------------------------------------------
function enqueue(task) {
  const run = state.queue.then(task);
  state.queue = run.catch(() => {});
  return run;
}

// Run a user action: clear the previous notice, then surface any engine
// rejection (unknown node, healing a healthy link, invalid seed, server error)
// and resync the view with the engine's actual state.
function runAction(task, { keepTimeline = false } = {}) {
  return enqueue(async () => {
    hideNotice();
    if (!keepTimeline) {
      forkTimeline();
    }
    try {
      await task();
    } catch (error) {
      showNotice(`Action rejected: ${errorMessage(error)}`);
      try {
        await refresh();
      } catch (_refreshError) {
        // The notice already reports the failure; keep the last good view.
      }
    }
  });
}

function errorMessage(error) {
  if (typeof error === "string") {
    return error;
  }
  return String(error?.message || error || "unknown error");
}

function showNotice(message, kind = "error") {
  el.notice.className = kind === "warn" ? "action-notice warn" : "action-notice";
  el.notice.textContent = message;
  el.notice.hidden = false;
}

function hideNotice() {
  el.notice.hidden = true;
  el.notice.textContent = "";
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
  state.nodeRows.clear();
  el.nodes.replaceChildren();
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
    return JSON.stringify(await this.get("/sim/replay"));
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
  el.banner.textContent = errorMessage(error);
  showNotice(errorMessage(error));
  for (const button of [el.step, el.play, el.copy, el.loadScenario, el.downloadScript, el.guideDo, ...el.untilButtons, ...el.linkActions]) {
    button.disabled = true;
  }
}

boot();
