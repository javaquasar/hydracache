import {
  MAX_SEED,
  parseSeed,
  readInitialState,
  reproducerCommand,
  snapshotHash,
  writeUrlState,
} from "./share.js";
import { SCENARIOS } from "./scenarios.js";
import { ServerSimSession, WasmSimSession } from "./engine.js";
import { initGraph, playDataFlowPulse, resetGraph, syncGraph } from "./graph.js";
import {
  graphSummary,
  keyAgreement,
  leaderOf,
  nodeStatus,
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
  shell: document.querySelector("[data-testid='demo-shell']"),
  linkSelect: document.querySelector("#link-select"),
  graphSummary: document.querySelector("#graph-summary"),
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
  initGraph(el.graph, {
    getSnapshot: () => state.snapshot,
    getSelectedLink: () => state.selectedLink,
    onSelectLink: (link) => {
      state.selectedLink = link;
      render();
    },
    onClientPress: (client) => void runAction(() => pushEventFor(client.id, client.namespace)),
  });
  bindPreferenceState();
  collapseSecondaryPanelsOnSmallScreens();
  setBusy(true);
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
    setBusy(false);
  });
}

// Controls that need a loaded engine stay disabled (and the page aria-busy)
// until the WASM package or the server session is ready. Per-state controls
// (link actions, guide, timeline) are then managed by render().
function engineControls() {
  return [
    el.step,
    el.play,
    el.copy,
    el.downloadScript,
    el.loadScenario,
    el.reset,
    el.addNode,
    el.subscribe,
    el.pushEvent,
    el.mode,
    el.workload,
    ...el.untilButtons,
  ];
}

function setBusy(busy) {
  el.shell.setAttribute("aria-busy", busy ? "true" : "false");
  for (const control of engineControls()) {
    control.disabled = busy;
  }
}

// On narrow screens the side panel stacks under the graph; start with the
// read-only panels collapsed so the controls a user acts on stay close.
function collapseSecondaryPanelsOnSmallScreens() {
  if (typeof window.matchMedia !== "function" || !window.matchMedia("(max-width: 900px)").matches) {
    return;
  }
  for (const panel of document.querySelectorAll("details[data-collapse-mobile]")) {
    panel.removeAttribute("open");
  }
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
  el.linkSelect.addEventListener("change", () => {
    const [from, to] = el.linkSelect.value.split(">");
    state.selectedLink = from && to ? { from, to } : null;
    render();
  });
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
  el.graphSummary.textContent = graphSummary(snapshot);
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

function renderSelectedLink() {
  renderLinkOptions();
  const link = selectedLinkView();
  el.selectedLink.textContent = link
    ? `${link.from} -> ${link.to} (${link.state})`
    : "none";
  for (const button of el.linkActions) {
    button.disabled = !link;
  }
}

// Keyboard/screen-reader route to the same link selection as clicking a graph
// edge. Options are only rebuilt when the set of links or their states change,
// so an open dropdown is not reset by auto-play.
function renderLinkOptions() {
  const links = state.snapshot?.links || [];
  const signature = links.map((link) => `${link.from}>${link.to}:${link.state}`).join(",");
  if (el.linkSelect.dataset.signature !== signature) {
    el.linkSelect.dataset.signature = signature;
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Choose a link";
    const options = links.map((link) => {
      const option = document.createElement("option");
      option.value = `${link.from}>${link.to}`;
      option.textContent = `${link.from} -> ${link.to} (${link.state})`;
      return option;
    });
    el.linkSelect.replaceChildren(placeholder, ...options);
  }
  const selected = state.selectedLink ? `${state.selectedLink.from}>${state.selectedLink.to}` : "";
  if (el.linkSelect.value !== selected) {
    el.linkSelect.value = selected;
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
    "Could an operation at each level be acknowledged right now? Computed with the product consistency rules: every node is a replica in one region and only live nodes acknowledge (isolated counts as down; single partitioned links are not considered). The simulator does not run per-level reads or writes.";
  const levels = document.createElement("div");
  levels.className = "metric-list";
  for (const view of snapshot.consistency || []) {
    const row = document.createElement("div");
    row.className = `metric consistency-row ${view.satisfiable ? "ok" : "bad"}`;
    row.dataset.level = view.level;
    const label = document.createElement("strong");
    label.textContent = view.level.toUpperCase();
    const status = document.createElement("span");
    status.textContent = `${view.satisfiable ? "can ack" : "cannot ack"} · needs ${view.required_acks} of ${view.total_replicas}, ${view.live_replicas} live`;
    row.append(label, status);
    if (view.reason) {
      const reason = document.createElement("small");
      reason.className = "consistency-reason";
      reason.textContent = view.reason;
      row.append(reason);
    }
    levels.append(row);
  }
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
  el.consistency.replaceChildren(note, levels, list);
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
  state.nodeRows.clear();
  el.nodes.replaceChildren();
  if (state.engine === "server") {
    const sim = new ServerSimSession(state.apiBase);
    await sim.reset(seed);
    return sim;
  }
  return new WasmSimSession(new state.SimHandle(BigInt(seed)));
}

function showEngineError(error) {
  el.verdict.className = "verdict warn";
  el.verdict.textContent =
    state.engine === "server" ? "server simulator unavailable" : "wasm package unavailable";
  el.banner.textContent = errorMessage(error);
  showNotice(errorMessage(error));
  el.shell.setAttribute("aria-busy", "false");
  for (const button of [el.step, el.play, el.copy, el.loadScenario, el.downloadScript, el.guideDo, ...el.untilButtons, ...el.linkActions]) {
    button.disabled = true;
  }
}

boot();
