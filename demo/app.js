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
};

const el = {
  banner: document.querySelector("#engine-banner"),
  verdict: document.querySelector("#verdict"),
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
    if (initial.scenario === "default") {
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
    await state.sim?.subscribe("client-a", el.manualNs.value);
    await refresh();
  });
  el.pushEvent.addEventListener("click", async () => {
    await state.sim?.push_event(
      "client-a",
      el.manualNs.value,
      el.manualKey.value,
      el.manualValue.value,
    );
    await refresh();
  });
  el.addNode.addEventListener("click", async () => {
    await state.sim?.add_node();
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
  writeUrlState(window.history, state.snapshot, state.scenario, state.engine, state.apiBase);
  render();
}

function render() {
  const snapshot = state.snapshot;
  el.banner.textContent = `This runs the real hydracache-sim engine, seed ${snapshot.seed}, step ${snapshot.step}. Formation ${snapshot.formation_phase}; election ${snapshot.election_source}. ${snapshot.election_disclosure || "Verdicts are produced by the actual invariant checker."}`;
  el.scenario.value = state.scenario;
  el.mode.value = snapshot.mode || "manual";
  el.interventionStatus.textContent =
    snapshot.intervention_count > 0
      ? `${snapshot.mode} · ${snapshot.intervention_count} replay action(s)`
      : `${snapshot.mode || "manual"} · no interventions`;
  renderVerdict(snapshot);
  renderGraph(snapshot);
  renderSelectedLink();
  renderProgress(snapshot);
  renderNodes(snapshot);
  renderSignals(snapshot);
  renderClients(snapshot);
  renderConsistency(snapshot);
  renderKeys(snapshot);
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

function renderGraph(snapshot) {
  el.graph.replaceChildren();
  const positions = layoutNodes(snapshot.nodes);
  const linkLayer = svg("g", { class: "links" });
  const packetLayer = svg("g", { class: "packet-layer", "aria-hidden": "true" });
  const nodeLayer = svg("g", { class: "nodes" });
  el.graph.append(linkLayer, packetLayer, nodeLayer);

  for (const link of snapshot.links) {
    const from = positions.get(link.from);
    const to = positions.get(link.to);
    if (!from || !to) {
      continue;
    }
    const selected = isSelectedLink(link);
    const className = ["link", link.state, selected ? "selected" : ""].filter(Boolean).join(" ");
    const visual = svg("line", {
      class: className,
      x1: from.x,
      y1: from.y,
      x2: to.x,
      y2: to.y,
      "data-from": link.from,
      "data-to": link.to,
    });
    const hit = svg("line", {
      class: "link-hit",
      x1: from.x,
      y1: from.y,
      x2: to.x,
      y2: to.y,
      "data-from": link.from,
      "data-to": link.to,
    });
    hit.addEventListener("click", () => {
      state.selectedLink = { from: link.from, to: link.to };
      render();
    });
    linkLayer.append(visual, hit);
  }

  renderPacketLayer(packetLayer, positions, snapshot);

  for (const node of snapshot.nodes) {
    const pos = positions.get(node.id);
    const group = svg("g", {
      class: ["node", node.crashed ? "crashed" : ""].filter(Boolean).join(" "),
      transform: `translate(${pos.x} ${pos.y})`,
    });
    group.append(
      svg("circle", { r: 46 }),
      svg("text", { y: -5 }, node.id),
      svg("text", { y: 17 }, node.crashed ? "crashed" : node.vote_state || node.role),
    );
    nodeLayer.append(group);
  }
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
    const row = document.createElement("div");
    row.className = "metric";
    const label = document.createElement("strong");
    label.textContent = `${node.id} ${node.crashed ? "crashed" : node.disabled ? "disabled" : node.vote_state || "up"}`;
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
      await refresh();
    });
    const isolate = nodeButton("Isolate", async () => {
      await state.sim.isolate_node(node.id);
      await refresh();
    });
    const rejoin = nodeButton("Rejoin", async () => {
      await state.sim.rejoin_node(node.id);
      await refresh();
    });
    const disable = nodeButton(node.disabled ? "Enable" : "Disable", async () => {
      if (node.disabled) {
        await state.sim.enable_node(node.id);
      } else {
        await state.sim.disable_node(node.id);
      }
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
  await refresh();
}

function togglePlay() {
  if (state.playing) {
    stopPlay();
  } else {
    startPlay();
  }
}

async function copyReproducer() {
  if (!state.snapshot) {
    return;
  }
  const command = reproducerCommand(state.snapshot.seed, state.snapshot.step);
  try {
    await navigator.clipboard?.writeText(command);
    el.copyStatus.textContent = command;
  } catch (_error) {
    el.copyStatus.textContent = command;
  }
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

function layoutNodes(nodes) {
  const center = { x: 400, y: 260 };
  const radius = Math.min(180, 110 + nodes.length * 12);
  const positions = new Map();
  nodes.forEach((node, index) => {
    const angle = -Math.PI / 2 + (index * Math.PI * 2) / Math.max(nodes.length, 1);
    positions.set(node.id, {
      x: center.x + Math.cos(angle) * radius,
      y: center.y + Math.sin(angle) * radius,
    });
  });
  return positions;
}

function renderPacketLayer(layer, positions, snapshot) {
  const messages = (snapshot.in_flight || []).slice(0, 16);
  for (const [index, message] of messages.entries()) {
    const from = positions.get(message.from);
    const to = positions.get(message.to);
    if (!from || !to) {
      continue;
    }
    const progress = packetProgress(snapshot.logical_time_millis, index);
    const x = from.x + (to.x - from.x) * progress;
    const y = from.y + (to.y - from.y) * progress;
    const kind = packetKind(message.kind);
    const trailStart = 0.18;
    const trailEnd = Math.max(trailStart, progress - 0.08);
    layer.append(
      svg("line", {
        class: `packet-trail ${kind}`,
        x1: from.x + (to.x - from.x) * trailStart,
        y1: from.y + (to.y - from.y) * trailStart,
        x2: from.x + (to.x - from.x) * trailEnd,
        y2: from.y + (to.y - from.y) * trailEnd,
      }),
      svg("circle", {
        class: `packet ${kind}`,
        cx: x,
        cy: y,
        r: 7,
      }),
    );
  }
}

function packetProgress(logicalMillis, index) {
  const phase = (Number(logicalMillis || 0) + index * 137) % 560;
  return 0.22 + phase / 1000;
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
