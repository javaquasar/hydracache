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
  sim: null,
  snapshot: null,
  selectedLink: null,
  scenario: "default",
  timer: null,
  playing: false,
};

const el = {
  banner: document.querySelector("#engine-banner"),
  verdict: document.querySelector("#verdict"),
  seedInput: document.querySelector("#seed-input"),
  scenario: document.querySelector("#scenario-select"),
  loadScenario: document.querySelector("#load-scenario"),
  reset: document.querySelector("#reset-button"),
  step: document.querySelector("#step-button"),
  play: document.querySelector("#play-button"),
  copy: document.querySelector("#copy-reproducer"),
  workload: document.querySelector("#workload-toggle"),
  speed: document.querySelector("#speed-input"),
  graph: document.querySelector("#cluster-graph"),
  selectedLink: document.querySelector("#selected-link"),
  progress: document.querySelector("#progress-panel"),
  hash: document.querySelector("#snapshot-hash"),
  nodes: document.querySelector("#nodes-panel"),
  consistency: document.querySelector("#consistency-panel"),
  keys: document.querySelector("#keys-panel"),
  linkActions: Array.from(document.querySelectorAll("[data-action]")),
};

async function boot() {
  const initial = readInitialState(window.location.search);
  state.scenario = initial.scenario;
  el.seedInput.value = String(initial.seed);
  populateScenarios(initial.scenario);
  bindEvents();
  try {
    const wasm = await import("./pkg/hydracache_sim_wasm.js");
    if (typeof wasm.default === "function") {
      await wasm.default();
    }
    state.SimHandle = wasm.SimHandle;
    if (initial.scenario === "default") {
      resetSimulation(initial.steps);
    } else {
      loadScenario(initial.scenario, initial.steps);
    }
  } catch (error) {
    showEngineError(error);
  }
}

function bindEvents() {
  el.reset.addEventListener("click", () => resetSimulation());
  el.loadScenario.addEventListener("click", () => loadScenario(el.scenario.value));
  el.step.addEventListener("click", () => {
    state.sim?.step();
    refresh();
  });
  el.play.addEventListener("click", togglePlay);
  el.copy.addEventListener("click", copyReproducer);
  el.workload.addEventListener("change", () => {
    state.sim?.set_workload_enabled(el.workload.checked);
    refresh();
  });
  el.speed.addEventListener("input", () => {
    if (state.playing) {
      stopPlay();
      startPlay();
    }
  });
  for (const button of el.linkActions) {
    button.addEventListener("click", () => applyLinkAction(button.dataset.action));
  }
}

function resetSimulation(steps = 0) {
  if (!state.SimHandle) {
    return;
  }
  stopPlay();
  state.scenario = "default";
  el.scenario.value = state.scenario;
  state.selectedLink = null;
  state.sim = new state.SimHandle(BigInt(readSeed()));
  state.sim.set_workload_enabled(el.workload.checked);
  if (steps > 0) {
    state.sim.run(BigInt(steps));
  }
  refresh();
}

function loadScenario(name, targetSteps = null) {
  if (!state.SimHandle) {
    return;
  }
  stopPlay();
  state.selectedLink = null;
  state.scenario = name || "default";
  el.scenario.value = state.scenario;
  if (state.scenario === "default") {
    resetSimulation(Number.isInteger(targetSteps) ? targetSteps : 0);
    return;
  }
  state.sim = new state.SimHandle(BigInt(0));
  state.sim.apply_scenario(state.scenario);
  if (Number.isInteger(targetSteps)) {
    const snapshot = JSON.parse(state.sim.snapshot_json());
    if (targetSteps > snapshot.step) {
      state.sim.run(BigInt(targetSteps - snapshot.step));
    }
  }
  refresh();
  el.seedInput.value = String(state.snapshot.seed);
}

function refresh() {
  if (!state.sim) {
    return;
  }
  state.snapshot = JSON.parse(state.sim.snapshot_json());
  writeUrlState(window.history, state.snapshot, state.scenario);
  render();
}

function render() {
  const snapshot = state.snapshot;
  el.banner.textContent = `real engine, seed ${snapshot.seed}, step ${snapshot.step}`;
  el.scenario.value = state.scenario;
  renderVerdict(snapshot);
  renderGraph(snapshot);
  renderSelectedLink();
  renderProgress(snapshot);
  renderNodes(snapshot);
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
  const nodeLayer = svg("g", { class: "nodes" });
  el.graph.append(linkLayer, nodeLayer);

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

  for (const node of snapshot.nodes) {
    const pos = positions.get(node.id);
    const group = svg("g", {
      class: ["node", node.crashed ? "crashed" : ""].filter(Boolean).join(" "),
      transform: `translate(${pos.x} ${pos.y})`,
    });
    group.append(
      svg("circle", { r: 46 }),
      svg("text", { y: -5 }, node.id),
      svg("text", { y: 17 }, node.crashed ? "crashed" : node.role),
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
    term("Convergence", progress.convergence),
    desc(progress.convergence),
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
    label.textContent = `${node.id} ${node.crashed ? "crashed" : "up"}`;
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = node.crashed ? "Restart" : "Crash";
    button.dataset.testid = node.crashed ? `restart-${node.id}` : `crash-${node.id}`;
    button.addEventListener("click", () => {
      if (node.crashed) {
        state.sim.restart_node(node.id);
      } else {
        state.sim.crash_node(node.id);
      }
      refresh();
    });
    row.append(label, button);
    list.append(row);
  }
  el.nodes.replaceChildren(list);
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

function applyLinkAction(action) {
  const link = selectedLinkView();
  if (!link || !state.sim) {
    return;
  }
  const delay = action === "delay" ? BigInt(250) : BigInt(0);
  state.sim.inject(action, link.from, link.to, delay);
  if (action === "delay" || action === "drop") {
    state.sim.step();
  }
  refresh();
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
    el.banner.textContent = command;
  } catch (_error) {
    el.banner.textContent = command;
  }
}

function startPlay() {
  if (!state.sim || state.playing) {
    return;
  }
  state.playing = true;
  el.play.textContent = "Pause";
  state.timer = window.setInterval(() => {
    state.sim.step();
    refresh();
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

function showEngineError(error) {
  el.verdict.className = "verdict warn";
  el.verdict.textContent = "wasm package unavailable";
  el.banner.textContent = String(error?.message || error);
  for (const button of [el.step, el.play, el.copy, el.loadScenario, ...el.linkActions]) {
    button.disabled = true;
  }
}

boot();
