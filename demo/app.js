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
import {
  fitGraph,
  refreshGraphFit,
  tidyGraph,
  initGraph,
  playDataFlowPulse,
  pushPlan,
  resetGraph,
  syncGraph,
  vertexClientPoint,
  zoomGraph,
} from "./graph.js";
import {
  graphSummary,
  groupEvents,
  keyAgreement,
  leaderOf,
  nextFreeName,
  nodeDetails,
  nodeStatus,
  predictionHolds,
  scriptAtStep,
  scriptFileName,
  fullVersion,
  shortVersion,
  shortcutFor,
  speedLabel,
  stepChanges,
  timelineEvents,
  timelineMarks,
  historyStrip,
  tourOutcome,
  verdictView,
  versionHue,
  violationFocus,
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
  presentFullscreen: false,
  // When the page saw each key's current version on each node: "key|node" ->
  // step. Kept per run (reset with the timeline).
  keyWrites: new Map(),
  changesExpanded: false,
  shownCommitted: null,
  committedAnimation: 0,
  SimHandle: null,
  activeTab: "cluster",
  // Last view signature each panel was painted from (see paintPanel).
  painted: new Map(),
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
  tour: freshTour("default"),
  // What changed since the previously rendered snapshot of the same run
  // (insight.stepChanges); null right after a new run is created.
  changes: null,
  freshRun: true,
  // Graph highlights derived from `changes` and the violation detail.
  marks: { changed: new Set(), notable: new Set(), implicated: new Set() },
  // Violation card: the violation the user closed (so it stays closed).
  violationDismissed: null,
  violationSignature: "",
  // Node whose details popover is open.
  popoverNode: null,
  popover: null,
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
  timelineHistory: document.querySelector("#timeline-history"),
  historySummary: document.querySelector("#history-summary"),
  historyLegend: document.querySelector("#history-legend"),
  timelineLabel: document.querySelector("#timeline-label"),
  timelineHelp: document.querySelector("#timeline-help"),
  timelineMarks: document.querySelector("#timeline-marks"),
  speedValue: document.querySelector("#speed-value"),
  themeToggle: document.querySelector("#theme-toggle"),
  zoomIn: document.querySelector("#zoom-in"),
  zoomOut: document.querySelector("#zoom-out"),
  zoomFit: document.querySelector("#zoom-fit"),
  tidyGraph: document.querySelector("#tidy-graph"),
  present: document.querySelector("#present-graph"),
  events: document.querySelector("#events-panel"),
  verdictPanel: document.querySelector("#verdict-panel"),
  guideTitle: document.querySelector("#guide-title"),
  guideText: document.querySelector("#guide-text"),
  guideBack: document.querySelector("#guide-back"),
  guideDo: document.querySelector("#guide-do"),
  guideNext: document.querySelector("#guide-next"),
  guideDots: document.querySelector("#guide-dots"),
  guideOutcome: document.querySelector("#guide-outcome"),
  guidePredict: document.querySelector("#guide-predict"),
  guidePredictQuestion: document.querySelector("#guide-predict-question"),
  guidePredictOptions: document.querySelector("#guide-predict-options"),
  guideRun: document.querySelector("#guide-run"),
  guideLink: document.querySelector("#guide-link"),
  guideFinale: document.querySelector("#guide-finale"),
  guideFinaleText: document.querySelector("#guide-finale-text"),
  guideNextTour: document.querySelector("#guide-next-tour"),
  guideCopyRun: document.querySelector("#guide-copy-run"),
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
  manualReader: document.querySelector("#manual-reader"),
  manualReaderNs: document.querySelector("#manual-reader-ns"),
  namespaceOptions: document.querySelector("#namespace-options"),
  addClient: document.querySelector("#add-client-button"),
  replicationModel: document.querySelector("#replication-model"),
  replicationFactor: document.querySelector("#replication-factor"),
  writeLevel: document.querySelector("#write-level"),
  applyReplication: document.querySelector("#apply-replication"),
  replicationPanel: document.querySelector("#replication-panel"),
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
  stepChanges: document.querySelector("#step-changes"),
  violationCard: document.querySelector("#violation-card"),
  nodePopover: document.querySelector("#node-popover"),
  graphStage: document.querySelector(".graph-stage"),
  intro: document.querySelector("#intro"),
  introDismiss: document.querySelector("#intro-dismiss"),
  showIntro: document.querySelector("#show-intro"),
  controls: document.querySelector(".controls"),
  tabs: Array.from(document.querySelectorAll("[role='tab'][data-tab]")),
  tabPanels: Array.from(document.querySelectorAll("[data-tabpanel]")),
  dock: document.querySelector("#run-dock"),
  dockPlay: document.querySelector("#dock-play"),
  dockStep: document.querySelector("#dock-step"),
  dockStepLabel: document.querySelector("#dock-step-label"),
  dockVerdict: document.querySelector("#dock-verdict"),
  shortcuts: document.querySelector("#shortcuts"),
  shortcutsClose: document.querySelector("#shortcuts-close"),
  showShortcuts: document.querySelector("#show-shortcuts"),
  announcer: document.querySelector("#announcer"),
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
    onSubscriberPress: (subscriber) => {
      selectTab("clients");
      spotlight(el.subscribers.querySelector(`[data-subscriber="${CSS.escape(subscriber.id)}"]`));
    },
    onNodePress: (id) => openNodePopover(id),
    getNodeMarks: () => state.marks,
    onPaint: () => positionNodePopover(),
    // Packets cover one step of their flight per play interval.
    getStepMillis: () => Number(el.speed.value),
    onAutoFitChange: (on) => renderFitState(on),
  });
  bindPreferenceState();
  bindTheme();
  bindIntro();
  bindLegend();
  trackControlBarHeight();
  buildNodePopover();
  bindTabs();
  bindDock();
  bindShortcuts();
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
    // A link to a guide step opens the guide on that step (?tourStep=N).
    if (initial.tourStep > 0) {
      moveTourTo(initial.tourStep - 1);
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
  syncDock();
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
  el.guideRun.addEventListener("click", () => void playTour());
  el.guideLink.addEventListener("click", () => void copyStepLink());
  el.guideNextTour.addEventListener(
    "click",
    () => void runAction(() => loadScenario(el.guideNextTour.dataset.scenario)),
  );
  el.guideCopyRun.addEventListener("click", () => void runAction(copyReproducer, { keepTimeline: true }));
  el.seedInput.addEventListener("input", () => validateSeedInput());
  el.subscribe.addEventListener("click", () =>
    void runAction(async () => {
      await state.sim?.subscribe(fieldValue(el.manualReader, "subscriber"), fieldValue(el.manualReaderNs, "namespace"));
      await settleAfterIntervention();
      await refresh();
      // Ready for the next one: suggest a free subscriber name.
      el.manualReader.value = nextFreeName("reader-", subscriberNames(state.snapshot));
    }),
  );
  el.addClient.addEventListener("click", () =>
    void runAction(async () => {
      await state.sim?.add_client(fieldValue(el.manualClient, "writer"), fieldValue(el.manualNs, "namespace"));
      await refresh();
      el.manualClient.value = nextFreeName("client-", (state.snapshot?.clients || []).map((client) => client.id));
    }),
  );
  el.applyReplication.addEventListener("click", () =>
    void runAction(async () => {
      await state.sim?.set_replication(
        el.replicationModel.value,
        Math.max(1, Number(el.replicationFactor.value) || 1),
        el.writeLevel.value,
      );
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
  el.zoomIn.addEventListener("click", () => zoomGraph(1.25));
  el.zoomOut.addEventListener("click", () => zoomGraph(1 / 1.25));
  el.zoomFit.addEventListener("click", () => fitGraph());
  el.tidyGraph.addEventListener("click", () => tidyGraph());
  el.present.addEventListener("click", () => setPresentation(!presenting()));
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && presenting() && !event.defaultPrevented && !el.shortcuts.open) {
      setPresentation(false);
    }
  });
  // Leaving browser full screen (its own Esc) leaves presentation mode too.
  document.addEventListener("fullscreenchange", () => {
    if (!document.fullscreenElement && presenting() && state.presentFullscreen) {
      state.presentFullscreen = false;
      setPresentation(false);
    }
  });
  renderSpeed();
  el.speed.addEventListener("input", () => {
    renderSpeed();
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
  const previous = state.freshRun ? null : state.snapshot;
  state.freshRun = false;
  state.snapshot = JSON.parse(await state.sim.snapshot_json());
  state.changes = stepChanges(previous, state.snapshot);
  const focus = violationFocus(state.snapshot);
  const wasHolding = !previous || previous.verdict?.status === "holding";
  render();
  if (focus && wasHolding && !state.timeline.rewound) {
    onViolationDetected(focus);
  }
  if (state.changes?.leader) {
    const { to } = state.changes.leader;
    announce(to ? `Leader is now ${to} at step ${state.snapshot.step}.` : `No leader at step ${state.snapshot.step}.`);
  }
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
      state.tour.index + 1,
    );
  } catch (error) {
    showNotice(`Share URL was not updated: ${errorMessage(error)}`, "warn");
  }
}

function render() {
  const snapshot = state.snapshot;
  renderRunFacts(snapshot);
  renderElectionSource(snapshot);
  el.scenario.value = state.scenario;
  el.mode.value = snapshot.mode || "manual";
  el.interventionStatus.textContent =
    snapshot.intervention_count > 0
      ? `${snapshot.mode} · ${snapshot.intervention_count} replay action(s)`
      : `${snapshot.mode || "manual"} · no interventions`;
  el.workload.checked = Boolean(snapshot.workload_enabled);
  recordObservation(snapshot);
  for (const change of state.changes?.keys ?? []) {
    state.keyWrites.set(`${change.key}|${change.node}`, snapshot.step);
  }
  const focus = violationFocus(snapshot);
  state.marks = {
    changed: new Set(state.changes?.nodes.keys() ?? []),
    // Role, term or membership changes: what the graph flashes for.
    notable: new Set(
      [...(state.changes?.nodes ?? new Map())]
        .filter(([, fields]) => fields.some((change) => ["status", "term", "added"].includes(change.field)))
        .map(([id]) => id),
    ),
    implicated: new Set(focus?.nodes ?? []),
  };
  renderVerdict(snapshot);
  renderViolationCard(focus);
  renderChanges();
  renderTimeline(snapshot);
  renderGuide();
  syncGraph(snapshot);
  el.graphSummary.textContent = graphSummary(snapshot);
  renderSelectedLink();
  renderProgress(snapshot);
  renderNodes(snapshot);
  renderTabPanels(snapshot);
  renderNodePopover();
  renderTabBadges(snapshot);
  syncDock();
}

// Short, scannable run facts under the title; the election disclosure is the
// tooltip (and the source chip's title) instead of a paragraph of prose.
function renderRunFacts(snapshot) {
  const facts = [
    `seed ${snapshot.seed}`,
    `step ${snapshot.step}`,
    `Formation ${snapshot.formation_phase}`,
    `election ${snapshot.election_source}`,
  ].map((text) => {
    const fact = document.createElement("span");
    fact.className = "fact";
    fact.textContent = text;
    return fact;
  });
  el.banner.replaceChildren(...facts);
  el.banner.title = snapshot.election_disclosure || "Verdicts are produced by the actual invariant checker.";
}

function renderSpeed() {
  el.speedValue.textContent = speedLabel(Number(el.speed.value));
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
    : "none selected — click a line on the graph or choose one above";
  el.selectedLink.classList.toggle("empty-state", !link);
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

// The Committed counter ticks up to its new value instead of jumping (at
// once under reduced motion, and whenever it goes down, e.g. on a rewind).
function countUp(element, value) {
  const from = state.shownCommitted;
  state.shownCommitted = value;
  window.cancelAnimationFrame(state.committedAnimation);
  if (from === null || value <= from || document.documentElement.dataset.reducedMotion === "true") {
    return element;
  }
  element.textContent = String(from);
  const start = performance.now();
  const tick = (now) => {
    const t = Math.min((now - start) / 450, 1);
    element.textContent = String(Math.round(from + (value - from) * (1 - (1 - t) * (1 - t))));
    if (t < 1) state.committedAnimation = window.requestAnimationFrame(tick);
  };
  state.committedAnimation = window.requestAnimationFrame(tick);
  return element;
}

function renderProgress(snapshot) {
  const progress = snapshot.progress;
  el.progress.replaceChildren(
    term("Step", snapshot.step),
    desc(snapshot.step),
    term("Logical time", snapshot.logical_time_millis),
    desc(`${snapshot.logical_time_millis} ms`),
    term("Committed", progress.committed_entries),
    countUp(desc(progress.committed_entries, state.changes?.committed ? "changed" : ""), progress.committed_entries),
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
    if (state.marks.changed.has(node.id)) {
      // Restart the flash when the row also changed on the previous step.
      void view.row.offsetWidth;
      view.row.classList.add("changed");
    }
    view.label.textContent = `${node.id} ${status}`;
    view.meta.textContent = `term ${node.term}; votes ${node.votes_received}; commit ${node.commit_index}`;
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
  const crash = nodeButton("Crash", () => void runNodeAction(id, "crash"));
  const isolate = nodeButton("Isolate", () => void runNodeAction(id, "isolate"));
  const rejoin = nodeButton("Rejoin", () => void runNodeAction(id, "rejoin"));
  const disable = nodeButton("Disable", () => void runNodeAction(id, "disable"));
  buttons.append(crash, isolate, rejoin, disable);
  row.append(label, meta, buttons);
  return { row, label, meta, crash, isolate, rejoin, disable };
}

// Node interventions shared by the Nodes panel and the node popover. Handlers
// read the node's state at click time, not at render time.
const NODE_ACTIONS = {
  crash: (node) => (node.crashed ? state.sim.restart_node(node.id) : state.sim.crash_node(node.id)),
  isolate: (node) => state.sim.isolate_node(node.id),
  rejoin: (node) => state.sim.rejoin_node(node.id),
  disable: (node) => (node.disabled ? state.sim.enable_node(node.id) : state.sim.disable_node(node.id)),
};

function runNodeAction(id, kind) {
  return runAction(async () => {
    const node = (state.snapshot?.nodes || []).find((candidate) => candidate.id === id);
    if (!node) {
      throw new Error(`node ${id} is no longer in the cluster`);
    }
    await NODE_ACTIONS[kind](node);
    await settleAfterIntervention();
    await refresh();
  });
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
  clients.className = "metric-list entity-list";
  for (const client of snapshot.clients || []) {
    const row = entityRow(client.id, `${client.namespace} · via ${client.connected_node ?? "—"}`, client.last_op || "no writes yet");
    row.dataset.client = client.id;
    const push = document.createElement("button");
    push.type = "button";
    push.textContent = "Push";
    push.setAttribute("data-testid", `push-${client.id}`);
    push.setAttribute("aria-label", `Push ${el.manualKey.value || "the key"} as ${client.id} to ${client.namespace}`);
    push.addEventListener("click", () => void runAction(() => pushEventFor(client.id, client.namespace)));
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "ghost";
    remove.textContent = "Remove";
    remove.setAttribute("data-testid", `remove-${client.id}`);
    remove.setAttribute("aria-label", `Remove writer ${client.id}`);
    remove.addEventListener("click", () =>
      void runAction(async () => {
        await state.sim?.remove_client(client.id);
        await refresh();
      }),
    );
    const actions = document.createElement("div");
    actions.className = "entity-actions";
    actions.append(push, remove);
    row.append(actions);
    clients.append(row);
  }
  if (!clients.childElementCount) {
    // The form right above is the action; the block only explains.
    clients.append(emptyState("No writers yet. A writer pushes values into one namespace; add one above, or push to create it."));
  }
  // A new client is admitted by a Raft metadata commit (ClientUpsert), so the
  // engine refuses new names while there is no leader. Say so before the
  // user hits the refusal; admitted clients keep writing.
  if (!(snapshot.nodes || []).some((node) => node.vote_state === "leader")) {
    const hint = emptyState(
      "No Raft leader yet: a new client joins through a leader commit, so adding one is refused until a leader is elected. Clients already admitted keep writing through their own member.",
    );
    hint.setAttribute("data-testid", "admission-hint");
    clients.prepend(hint);
  }

  const subscribers = document.createElement("div");
  subscribers.className = "metric-list entity-list";
  for (const subscriber of snapshot.subscribers || []) {
    const event = subscriber.last_event
      ? `${subscriber.last_event.kind} ${subscriber.last_event.key} @ step ${subscriber.last_event.delivered_at_step}`
      : "waiting for events";
    const row = entityRow(
      subscriber.id,
      `via ${subscriber.connected_node ?? "—"} · lag ${subscriber.lag} · dropped ${subscriber.dropped}`,
      event,
    );
    row.dataset.subscriber = subscriber.id;
    const leave = document.createElement("button");
    leave.type = "button";
    leave.className = "ghost";
    leave.textContent = "Unsubscribe";
    leave.setAttribute("data-testid", `unsubscribe-${subscriber.id}`);
    leave.setAttribute("aria-label", `Unsubscribe ${subscriber.client_id} from ${subscriber.namespace}`);
    leave.addEventListener("click", () =>
      void runAction(async () => {
        await state.sim?.unsubscribe(subscriber.client_id, subscriber.namespace);
        await refresh();
      }),
    );
    row.append(leave);
    subscribers.append(row);
  }
  if (!subscribers.childElementCount) {
    subscribers.append(emptyState("No subscribers yet. A subscriber receives the committed events of one namespace; subscribe one above."));
  }
  el.clients.replaceChildren(clients);
  el.subscribers.replaceChildren(subscribers);
  renderNamespaceOptions(snapshot);
}

// An empty block says what would appear there and offers the action that
// makes it appear.
// The write path manual pushes follow and the recent routed writes, with the
// acknowledgements the coordinator counted (snapshot schema 11).
function renderReplication(snapshot) {
  const replication = snapshot.replication;
  if (!replication) {
    el.replicationPanel.replaceChildren();
    return;
  }
  el.replicationModel.value = replication.model;
  if (replication.replication_factor) el.replicationFactor.value = String(replication.replication_factor);
  if (replication.write_level && [...el.writeLevel.options].some((option) => option.value === replication.write_level)) {
    el.writeLevel.value = replication.write_level;
  }
  const note = document.createElement("p");
  note.className = "panel-note";
  note.setAttribute("data-testid", "replication-disclosure");
  note.textContent = replication.disclosure;
  const list = document.createElement("div");
  list.className = "metric-list entity-list";
  for (const write of (snapshot.writes || []).slice(-6).reverse()) {
    const row = document.createElement("div");
    row.className = `entity-row write-row status-${write.status}`;
    row.dataset.write = String(write.id);
    const title = document.createElement("strong");
    title.textContent = `#${write.id} ${write.key}`;
    const path = write.primary === write.coordinator ? write.coordinator : `${write.coordinator} → ${write.primary}`;
    const backups = write.backups.length ? ` → ${write.backups.join(", ")}` : "";
    const meta = document.createElement("span");
    meta.className = "entity-meta";
    const level = write.level ? ` at ${write.level.toUpperCase()}` : "";
    const hinted = (write.hinted || []).length ? ` · hint held for ${write.hinted.join(", ")}` : "";
    meta.textContent = `${write.client}: ${path}${backups} · acks ${write.acks}/${write.required_acks}${level} · ${write.status}${hinted}`;
    row.append(title, meta);
    if (write.reason) {
      const reason = document.createElement("span");
      reason.className = "entity-meta write-reason";
      reason.textContent = write.reason;
      row.append(reason);
    }
    list.append(row);
  }
  if (!list.childElementCount) {
    list.append(
      emptyState(
        replication.model === "full_broadcast"
          ? "Writes are broadcast to every member in this model, so there is no per-write record."
          : "No routed writes yet. Push a value to see its path and acknowledgements.",
      ),
    );
  }
  const parts = [note];
  const repair = snapshot.repair;
  if (repair && replication.model !== "full_broadcast") {
    // Hinted handoff and anti-entropy are simulator models of the product
    // rules (schema 12); say so next to the counters.
    const counters = document.createElement("p");
    counters.className = "panel-note";
    counters.setAttribute("data-testid", "repair-counters");
    const held = (snapshot.hints || []).length;
    counters.textContent =
      `Hints held ${held} · replayed ${repair.hints_replayed} · suppressed ${repair.hints_suppressed} · dropped ${repair.hints_dropped}` +
      ` · repair copies ${repair.repair_copies} (a pass every ${repair.interval_steps} steps${repair.last_run_step == null ? "" : `, last at step ${repair.last_run_step}`}).`;
    counters.title = repair.disclosure;
    parts.push(counters);
  }
  parts.push(list);
  el.replicationPanel.replaceChildren(...parts);
}

function emptyState(text, actionLabel, testId, onAction) {
  const box = document.createElement("div");
  box.className = "empty-state empty-action";
  const note = document.createElement("p");
  note.textContent = text;
  const action = document.createElement("button");
  action.type = "button";
  action.className = "ghost";
  action.textContent = actionLabel;
  action.setAttribute("data-testid", testId);
  action.addEventListener("click", onAction);
  box.append(note);
  if (actionLabel) box.append(action);
  return box;
}

function entityRow(name, meta, detail) {
  const row = document.createElement("div");
  row.className = "metric entity-row";
  const text = document.createElement("div");
  const label = document.createElement("strong");
  label.textContent = name;
  const where = document.createElement("span");
  where.className = "entity-meta";
  where.textContent = meta;
  const last = document.createElement("span");
  last.className = "entity-detail";
  last.textContent = detail;
  text.append(label, where, last);
  row.append(text);
  return row;
}

// Namespace fields suggest the namespaces already in use (free text allowed).
function renderNamespaceOptions(snapshot) {
  const names = new Set(["profiles"]);
  for (const client of snapshot.clients || []) names.add(client.namespace);
  for (const subscriber of snapshot.subscribers || []) names.add(subscriber.namespace);
  el.namespaceOptions.replaceChildren(
    ...[...names].sort().map((name) => {
      const option = document.createElement("option");
      option.value = name;
      return option;
    }),
  );
}

// The simulator is one fully replicated Raft group: per-request consistency
// levels (ONE ... ALL) are not modeled, so none are shown. Only engine-backed
// values appear here.
function renderConsistency(snapshot) {
  const note = document.createElement("p");
  note.className = "model-note";
  // Schema 13: in the replicated write path each key has its own owners, so
  // the engine computes the panel on the owners of the last written key.
  const scope = snapshot.consistency_scope || "every node is a replica";
  note.setAttribute("data-testid", "consistency-scope");
  note.textContent = `Could an operation at each level be acknowledged right now? Computed with the product consistency rules on: ${scope}, in one region; only live nodes acknowledge (isolated counts as down; single partitioned links are not considered). Reads are not run per level; writes are, in the replicated write path.`;
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

// Keys heatmap: one row per sampled key, one column per node. A cell's colour
// is derived from the stored checksum (equal checksums share a colour); live
// replicas that differ from the majority are outlined, down nodes striped.
const MAX_KEY_ROWS = 8;

function renderKeys(snapshot) {
  const agreement = keyAgreement(snapshot);
  const keys = agreement.keys.slice(0, MAX_KEY_ROWS);
  if (keys.length === 0) {
    el.keys.replaceChildren(
      emptyState("No sampled keys yet. They appear once the workload or a push writes a value.", "Push a value", "empty-push", () =>
        void runAction(() => pushEventFor(manualClientId(), (el.manualNs.value || "profiles").trim())),
      ),
    );
    return;
  }
  const changedKeys = new Map((state.changes?.keys ?? []).map((change) => [`${change.key}|${change.node}`, change]));
  const nodes = (snapshot.nodes || []).map((node) => node.id);
  const table = document.createElement("table");
  table.className = "key-grid";
  const caption = document.createElement("caption");
  caption.className = "visually-hidden";
  caption.textContent = "Stored checksum of each sampled key on each node";
  const head = document.createElement("tr");
  const corner = document.createElement("th");
  corner.scope = "col";
  corner.textContent = "key";
  head.append(corner);
  for (const id of nodes) {
    const th = document.createElement("th");
    th.scope = "col";
    th.textContent = id;
    th.title = id;
    head.append(th);
  }
  const statusHead = document.createElement("th");
  statusHead.scope = "col";
  statusHead.textContent = "state";
  head.append(statusHead);
  const thead = document.createElement("thead");
  thead.append(head);
  const tbody = document.createElement("tbody");
  for (const key of keys) {
    const row = document.createElement("tr");
    row.className = `key-line${key.agree ? "" : " diverged"}`;
    const name = document.createElement("th");
    name.scope = "row";
    name.textContent = key.key;
    name.title = key.key;
    row.append(name);
    for (const cell of key.cells) {
      const td = document.createElement("td");
      const chip = document.createElement("span");
      const change = changedKeys.get(`${key.key}|${cell.node}`);
      const fromHue = change && change.before !== null && change.before !== undefined ? versionHue(change.before) : null;
      // A replica that took a new value fades from its old colour to the new
      // one; one that had no value yet flashes.
      chip.className = [
        "replica",
        cell.owner === false ? "not-owner" : "",
        cell.primary ? "primary-owner" : "",
        cell.version === null ? "missing" : "",
        cell.diverged ? "diverged" : "",
        cell.live ? "" : "down",
        change ? (fromHue !== null && cell.version !== null ? "recolour" : "changed") : "",
      ]
        .filter(Boolean)
        .join(" ");
      const hue = versionHue(cell.version);
      if (hue !== null) {
        chip.style.setProperty("--hue", String(hue));
      }
      if (fromHue !== null) {
        chip.style.setProperty("--from-hue", String(fromHue));
      }
      chip.textContent = shortVersion(cell.version).slice(-4);
      const downNote = cell.live ? "" : " (down)";
      const written = state.keyWrites.get(`${key.key}|${cell.node}`);
      const when = written === undefined ? "unchanged since this run was opened" : `written at step ${written}`;
      const ownerNote = cell.owner === false ? " (not an owner)" : cell.primary ? " (primary owner)" : "";
      chip.title =
        cell.version === null
          ? `${cell.node}: no replica of ${key.key}${ownerNote}${downNote}`
          : `${cell.node} · ${key.key}\nchecksum ${fullVersion(cell.version)}\n${when}${cell.diverged ? "\ndiffers from the live majority" : ""}${downNote}`;
      chip.dataset.written = written === undefined ? "" : String(written);
      chip.setAttribute("aria-label", chip.title.replaceAll("\n", ", "));
      td.append(chip);
      row.append(td);
    }
    const status = document.createElement("td");
    status.className = "key-state";
    status.textContent = key.agree ? "agree" : key.majority === null ? "no copy" : "replicas differ";
    row.append(status);
    tbody.append(row);
  }
  table.append(caption, thead, tbody);
  const wrap = document.createElement("div");
  wrap.className = "key-grid-wrap";
  wrap.append(table);
  const legend = document.createElement("p");
  legend.className = "model-note";
  const more = agreement.keys.length - keys.length;
  legend.textContent =
    `Same colour = same stored checksum (last 4 hex digits shown). Outlined = differs from the live majority; striped = node down; — = no replica.` +
    (more > 0 ? ` ${more} more key(s) not shown.` : "");
  el.keys.replaceChildren(wrap, legend);
}

// ---------------------------------------------------------------------------
// Timeline: every earlier step of the run is rebuilt by replaying its script
// prefix (interventions up to that step, resumed exactly there).
// ---------------------------------------------------------------------------
function startTimeline(scenario, base) {
  state.timeline = { script: null, base, furthest: base, rewound: false };
  state.observations = new Map();
  state.keyWrites = new Map();
  state.shownCommitted = null;
  state.tour = freshTour(scenario);
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
    committed: snapshot.progress?.committed_entries ?? null,
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
  const allEvents = timelineEvents(script, state.observations);
  const markEvents = [...allEvents];
  const first = snapshot.first_violation_step;
  if (first !== null && first !== undefined) {
    markEvents.push({ step: first, kind: "verdict", text: "first violation reported by the checker" });
  }
  renderTimelineMarks(markEvents, base, max);
  renderHistoryStrip(snapshot, base, max);
  el.timelineHelp.textContent = rewound
    ? `Rewound to step ${snapshot.step} of a recording that reaches step ${max}. Drag forward to replay it; Step, Play or any action continues from here and drops the later recording.`
    : "Drag to rebuild any earlier step of this run from its seed and recorded actions.";
  const events = allEvents.filter((event) => event.step >= base && event.step <= max);
  const groups = groupEvents(events, snapshot.step).slice(-MAX_TIMELINE_EVENTS);
  paintPanel("events", null, JSON.stringify(groups), () => renderEventGroups(groups));
}

function renderEventGroups(groups) {
  const items = groups.map((group) => {
    const item = document.createElement("li");
    item.className = ["event", "event-group", group.current ? "current" : "", group.future ? "future" : ""]
      .filter(Boolean)
      .join(" ");
    const button = document.createElement("button");
    button.type = "button";
    button.title = `Show step ${group.step}`;
    if (group.current) {
      button.setAttribute("aria-current", "step");
    }
    const stepChip = document.createElement("span");
    stepChip.className = "event-step";
    stepChip.textContent = `step ${group.step}`;
    button.append(stepChip);
    for (const entry of group.items) {
      const tag = document.createElement("span");
      tag.className = `event-item ${entry.kind}`;
      const icon = document.createElement("span");
      icon.className = "event-icon";
      icon.setAttribute("aria-hidden", "true");
      icon.textContent = entry.icon;
      tag.append(icon, entry.text);
      button.append(tag);
    }
    button.addEventListener("click", () =>
      void runAction(() => rewindTo(group.step), { keepTimeline: true }),
    );
    item.append(button);
    return item;
  });
  if (items.length === 0) {
    const empty = document.createElement("li");
    empty.className = "event empty";
    empty.textContent = "no events yet — interventions, leader changes, and verdict changes appear here";
    const run = document.createElement("button");
    run.type = "button";
    run.className = "ghost";
    run.textContent = "Run to leader";
    run.setAttribute("data-testid", "empty-run-leader");
    run.addEventListener("click", () => el.untilButtons.find((button) => button.dataset.until === "leader")?.click());
    empty.append(" ", run);
    items.push(empty);
  }
  el.events.replaceChildren(...items);
}

// Event ticks over the scrubber: where the leader changed, the verdict flipped,
// or a recorded action happened.
// The strip under the scrubber: leader per observed step (one colour per
// node, red for no leader, blank where the page did not see the step) and a
// line of committed entries.
function renderHistoryStrip(snapshot, base, max) {
  const strip = historyStrip(state.observations, base, max);
  const order = (snapshot.nodes || []).map((node) => node.id).sort((a, b) => a.localeCompare(b, "en", { numeric: true }));
  const hue = (leader) => (Math.max(0, order.indexOf(leader)) * 137.5 + 170) % 360;
  paintPanel("history", null, JSON.stringify([strip, order]), () => {
    const NS = "http://www.w3.org/2000/svg";
    const shapes = strip.segments.map((segment) => {
      const rect = document.createElementNS(NS, "rect");
      rect.setAttribute("x", segment.x0.toFixed(1));
      rect.setAttribute("y", "15");
      rect.setAttribute("width", Math.max(1, segment.x1 - segment.x0).toFixed(1));
      rect.setAttribute("height", "9");
      rect.setAttribute("class", segment.leader ? "history-leader" : "history-none");
      if (segment.leader) rect.style.fill = `hsl(${hue(segment.leader)} 55% 46%)`;
      rect.dataset.leader = segment.leader ?? "";
      rect.dataset.steps = `${segment.from}-${segment.to}`;
      return rect;
    });
    for (const points of strip.lines) {
      const line = document.createElementNS(NS, "polyline");
      line.setAttribute("points", points);
      line.setAttribute("class", "history-committed");
      shapes.push(line);
    }
    el.timelineHistory.replaceChildren(...shapes);
    el.historySummary.textContent = strip.summary;
    const chips = strip.leaders.map((leader) => {
      const chip = document.createElement("span");
      chip.className = leader ? "history-chip" : "history-chip none";
      if (leader) chip.style.setProperty("--chip", `hsl(${hue(leader)} 55% 46%)`);
      chip.textContent = leader ?? "no leader";
      return chip;
    });
    const committed = document.createElement("span");
    committed.className = "history-chip committed";
    committed.textContent = "committed";
    el.historyLegend.replaceChildren(...chips, ...(strip.lines.length ? [committed] : []));
  });
}

function renderTimelineMarks(events, base, max) {
  const views = timelineMarks(events, base, max);
  paintPanel("timeline-marks", null, JSON.stringify(views), () => paintTimelineMarks(views));
}

function paintTimelineMarks(views) {
  const marks = views.map((mark) => {
    const tick = document.createElement("span");
    tick.className = `timeline-mark ${mark.kind}`;
    tick.style.left = `${mark.percent}%`;
    tick.title = mark.text;
    return tick;
  });
  el.timelineMarks.replaceChildren(...marks);
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
// Guided tours (scenarios.js): narration plus optional engine actions. After
// "Do it" the guide reads the result back from the snapshot (tourOutcome) and
// marks the step done, or says plainly that its claim did not happen. A step
// may ask for a prediction first; the right answer is whatever the engine did.
// ---------------------------------------------------------------------------
const TOUR_PAUSE_MS = 1600;

function freshTour(scenario) {
  return { scenario, index: 0, crashed: [], results: new Map(), predictions: new Map(), running: false };
}

function currentTour() {
  return SCENARIOS.find((scenario) => scenario.name === state.tour.scenario)?.tour ?? [];
}

function nextTourScenario() {
  const withTours = SCENARIOS.filter((scenario) => scenario.tour?.length);
  const at = withTours.findIndex((scenario) => scenario.name === state.tour.scenario);
  return withTours[(at + 1) % withTours.length];
}

function renderGuide() {
  const tour = currentTour();
  const scenario = SCENARIOS.find((candidate) => candidate.name === state.tour.scenario);
  const index = Math.min(state.tour.index, Math.max(tour.length - 1, 0));
  const step = tour[index];
  const result = state.tour.results.get(index);
  el.guideTitle.textContent = tour.length
    ? `Guide · ${scenario.title} · ${index + 1}/${tour.length}`
    : "Guide";
  el.guideText.textContent = step?.text ?? "No guide for this run.";
  renderGuideDots(tour, index);
  renderGuidePrediction(step, index, result);
  el.guideOutcome.hidden = !result;
  if (result) {
    el.guideOutcome.className = `guide-outcome ${result.ok ? "ok" : "bad"}`;
    const main = document.createElement("span");
    main.textContent = `${result.ok ? "✓" : "✗"} ${result.text}`;
    el.guideOutcome.replaceChildren(main);
    // The prediction is about the reader, not the engine: its own line.
    if (result.prediction) {
      const guess = document.createElement("span");
      guess.className = `guide-guess ${result.guessRight ? "right" : "wrong"}`;
      guess.textContent = result.prediction;
      el.guideOutcome.append(guess);
    }
  }
  const busy = state.tour.running;
  el.guideBack.disabled = index === 0 || busy;
  el.guideNext.disabled = index >= tour.length - 1 || busy;
  el.guideDo.disabled = !step?.actions?.length || Boolean(result?.ok) || busy;
  // Once a step is done, Next becomes the obvious way on.
  el.guideNext.classList.toggle("primary", Boolean(result?.ok));
  el.guideDo.classList.toggle("primary", !result?.ok);
  el.guideRun.textContent = busy ? "Stop the tour" : "Play the tour";
  el.guideRun.disabled = tour.length === 0;
  renderGuideFinale(tour, index);
}

function renderGuideDots(tour, index) {
  const dots = tour.map((_step, at) => {
    const result = state.tour.results.get(at);
    const dot = document.createElement("li");
    const button = document.createElement("button");
    button.type = "button";
    button.className = ["guide-dot", result ? (result.ok ? "done" : "failed") : "", at === index ? "current" : ""]
      .filter(Boolean)
      .join(" ");
    const status = result ? (result.ok ? ", done" : ", did not happen") : "";
    button.setAttribute("aria-label", `Step ${at + 1} of ${tour.length}${status}`);
    if (at === index) button.setAttribute("aria-current", "step");
    button.disabled = state.tour.running;
    button.addEventListener("click", () => moveTourTo(at));
    dot.append(button);
    return dot;
  });
  el.guideDots.replaceChildren(...dots);
}

function renderGuidePrediction(step, index, result) {
  const predict = step?.predict;
  el.guidePredict.hidden = !predict;
  if (!predict) return;
  const chosen = state.tour.predictions.get(index);
  el.guidePredictQuestion.textContent = predict.question;
  const buttons = predict.options.map((option, at) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "guide-option";
    button.textContent = option.label;
    button.setAttribute("aria-pressed", chosen === at ? "true" : "false");
    // After the step ran, mark what actually happened.
    if (result?.actual === at) button.classList.add("actual");
    else if (result && chosen === at) button.classList.add("wrong");
    button.disabled = Boolean(result) || state.tour.running;
    button.addEventListener("click", () => {
      state.tour.predictions.set(index, at);
      renderGuide();
    });
    return button;
  });
  el.guidePredictOptions.replaceChildren(...buttons);
}

function renderGuideFinale(tour, index) {
  const last = tour.length - 1;
  const finished = tour.length > 0 && index === last && state.tour.results.get(last)?.ok;
  el.guideFinale.hidden = !finished;
  if (!finished) return;
  const checked = [...state.tour.results.values()].filter((result) => result.ok).length;
  const reported = tour.filter((step) => step.report).length;
  el.guideFinaleText.textContent = `Tour complete: ${checked} of ${reported} checked steps confirmed by the engine.`;
  const next = nextTourScenario();
  el.guideNextTour.textContent = `Next tour: ${next.title}`;
  el.guideNextTour.dataset.scenario = next.name;
}

function moveTour(delta) {
  moveTourTo(state.tour.index + delta);
}

function moveTourTo(target) {
  const tour = currentTour();
  state.tour.index = Math.min(Math.max(target, 0), Math.max(tour.length - 1, 0));
  // Open the tab the step is about, so its text and the panel line up.
  const focus = tour[state.tour.index]?.focus;
  if (focus?.tab) selectTab(focus.tab);
  renderGuide();
  scheduleUrlWrite();
}

// Show where a step's result is: open its tab and spotlight the element.
function showTourFocus(focus) {
  if (!focus) return;
  if (focus.tab) selectTab(focus.tab);
  if (focus.target) spotlight(document.querySelector(focus.target));
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
    const last = state.tour.crashed.at(-1);
    if (!last) {
      throw new Error("this guide has not crashed a node yet");
    }
    return last;
  }
  return node;
}

async function runTourStep() {
  const tour = currentTour();
  const index = state.tour.index;
  const step = tour[index];
  if (!step?.actions?.length) return;
  const before = state.snapshot;
  try {
    for (const action of step.actions) {
      await runTourAction(action);
      await refresh();
    }
  } catch (error) {
    state.tour.results.set(index, { ok: false, text: `The step stopped: ${errorMessage(error)}` });
    renderGuide();
    throw error;
  }
  const result = step.report
    ? tourOutcome(step.report, state.snapshot, { before, crashed: state.tour.crashed.at(-1) })
    : { ok: true, text: `Done at step ${state.snapshot.step}.` };
  if (step.predict) {
    const actual = step.predict.options.findIndex((option) => predictionHolds(option.when, state.snapshot));
    const chosen = state.tour.predictions.get(index);
    result.actual = actual;
    if (chosen !== undefined && actual >= 0) {
      result.guessRight = chosen === actual;
      result.prediction =
        chosen === actual
          ? "Your prediction was right."
          : `You predicted "${step.predict.options[chosen].label}"; the engine did "${step.predict.options[actual].label}".`;
    }
  }
  state.tour.results.set(index, result);
  renderGuide();
  showTourFocus(step.focus);
  announce(`Guide step ${index + 1}: ${result.ok ? "done" : "did not happen"}. ${result.text}`);
}

async function runTourAction(action) {
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
      state.tour.crashed.push(node);
      await settleAfterIntervention();
      break;
    }
    case "restart": {
      const nodes = action.node === "$all_crashed" ? [...state.tour.crashed] : [tourNode(action.node)];
      if (nodes.length === 0) {
        throw new Error("this guide has not crashed a node yet");
      }
      for (const node of nodes) {
        await state.sim.restart_node(node);
      }
      state.tour.crashed = state.tour.crashed.filter((node) => !nodes.includes(node));
      await settleAfterIntervention();
      break;
    }
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
      await pushWithPulse(action.client, action.ns, action.key, action.value);
      break;
    default:
      throw new Error(`unknown guide action '${action.type}'`);
  }
}

// Play the tour: run each remaining step, pause on its result, move on. It
// stops on a step whose claim did not happen, or when the user stops it.
async function playTour() {
  if (state.tour.running) {
    state.tour.running = false;
    renderGuide();
    return;
  }
  state.tour.running = true;
  renderGuide();
  const scenario = state.tour.scenario;
  const pause = () => new Promise((resolve) => window.setTimeout(resolve, TOUR_PAUSE_MS));
  try {
    for (;;) {
      const tour = currentTour();
      const index = state.tour.index;
      const step = tour[index];
      if (step?.actions?.length && !state.tour.results.get(index)?.ok) {
        await runAction(runTourStep);
      }
      if (!state.tour.running || state.tour.scenario !== scenario) break;
      if (step?.actions?.length && !state.tour.results.get(index)?.ok) break;
      if (index >= tour.length - 1) break;
      await pause();
      if (!state.tour.running || state.tour.scenario !== scenario) break;
      moveTourTo(index + 1);
      await pause();
    }
  } finally {
    state.tour.running = false;
    renderGuide();
  }
}

// The page URL already carries the run (seed, script) and the guide step.
async function copyStepLink() {
  await enqueue(writeShareUrl);
  const link = window.location.href;
  try {
    await navigator.clipboard?.writeText(link);
  } catch (_error) {
    // Clipboard blocked: the link is still shown below.
  }
  el.copyStatus.textContent = link;
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
// A required text field: trimmed, and an empty one fails loud with its name.
function fieldValue(input, name) {
  const value = (input?.value || "").trim();
  if (!value) {
    throw new Error(`enter a ${name} name`);
  }
  return value;
}

function subscriberNames(snapshot) {
  return (snapshot?.subscribers || []).map((subscriber) => subscriber.client_id);
}

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
  await pushWithPulse(client, namespace, el.manualKey.value, el.manualValue.value);
  await refresh();
}

// Push, let the cluster settle, then show the data flow the engine actually
// ran: the snapshot right after the push gives the push step and entry node,
// the settled one when each replica applied the value (applied_at_step),
// which copies are still on their way or lost, and when subscribers heard.
// Graph vertices flash as their copy arrives (graph.js flashWhenReached).
async function pushWithPulse(client, namespace, key, value) {
  await state.sim.push_event(client, namespace, key, value);
  const after = JSON.parse(await state.sim.snapshot_json());
  await settleAfterIntervention();
  const final = JSON.parse(await state.sim.snapshot_json());
  // The animation replays what the engine did (pushPlan); the render that
  // follows schedules each vertex's flash for when its copy arrives.
  playDataFlowPulse(pushPlan(after, final, client, namespace, key));
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
    // A routed write takes several steps (forward, copies, acknowledgements);
    // keep going while one is pending, within the same bound.
    const writePending = (snapshot.writes || []).some((write) => write.status === "pending");
    if (hasLeader && !stillJoining && !writePending) {
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
  syncDock();
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
  syncDock();
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

function desc(text, className = "") {
  const node = document.createElement("dd");
  node.textContent = String(text);
  if (className) {
    node.className = className;
  }
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

// Theme: an explicit choice (stored) wins; otherwise follow the OS setting.
const THEME_KEY = "hydracache-demo-theme";
let chosenTheme = null;

function storedTheme() {
  try {
    const value = window.localStorage.getItem(THEME_KEY);
    return value === "dark" || value === "light" ? value : null;
  } catch (_error) {
    return null;
  }
}

function bindTheme() {
  const media =
    typeof window.matchMedia === "function" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
  const apply = () => {
    const theme = chosenTheme ?? storedTheme() ?? (media?.matches ? "dark" : "light");
    document.documentElement.dataset.theme = theme;
    const next = theme === "dark" ? "light" : "dark";
    el.themeToggle.setAttribute("aria-label", `Use ${next} theme`);
    el.themeToggle.title = `Use ${next} theme`;
    el.themeToggle.setAttribute("aria-pressed", theme === "dark" ? "true" : "false");
  };
  apply();
  media?.addEventListener?.("change", apply);
  el.themeToggle.addEventListener("click", () => {
    const next = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    chosenTheme = next;
    try {
      window.localStorage.setItem(THEME_KEY, next);
    } catch (_error) {
      // Storage can be unavailable (private mode); the choice still applies now.
    }
    apply();
  });
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
  state.freshRun = true;
  state.changes = null;
  state.violationDismissed = null;
  closeNodePopover({ restoreFocus: false });
  state.nodeRows.clear();
  state.painted.clear();
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

// ---------------------------------------------------------------------------
// What changed since the previous view (engine fields only).
// ---------------------------------------------------------------------------
const MAX_CHANGE_ITEMS = 6;

function renderChanges() {
  const changes = state.changes;
  if (!changes) {
    el.stepChanges.textContent = "";
    el.stepChanges.hidden = true;
    return;
  }
  el.stepChanges.hidden = false;
  if (changes.summary.length === 0) {
    el.stepChanges.textContent = "No engine state changed since the previous view.";
    el.stepChanges.classList.add("quiet");
    return;
  }
  el.stepChanges.classList.remove("quiet");
  const expanded = state.changesExpanded;
  const shown = expanded ? changes.summary : changes.summary.slice(0, MAX_CHANGE_ITEMS);
  const more = changes.summary.length - Math.min(changes.summary.length, MAX_CHANGE_ITEMS);
  const label = document.createElement("strong");
  label.textContent = "Changed: ";
  const parts = [label, shown.join(" · ")];
  if (more > 0) {
    // The rest of the list on demand; the choice holds for later steps.
    const toggle = document.createElement("button");
    toggle.type = "button";
    toggle.className = "link-button";
    toggle.setAttribute("data-testid", "step-changes-toggle");
    toggle.setAttribute("aria-expanded", expanded ? "true" : "false");
    toggle.textContent = expanded ? "show less" : `+${more} more`;
    toggle.addEventListener("click", () => {
      state.changesExpanded = !state.changesExpanded;
      renderChanges();
    });
    parts.push(" · ", toggle);
  }
  el.stepChanges.replaceChildren(...parts);
}

// ---------------------------------------------------------------------------
// Violation: pause on the transition, jump to the first violating step, and
// say what broke and how to reproduce it. Everything shown comes from the
// engine's verdict; highlighted nodes are the ids its detail text names.
// ---------------------------------------------------------------------------
function onViolationDetected(focus) {
  stopPlay();
  state.violationDismissed = null;
  renderViolationCard(focus);
  const { base } = state.timeline;
  if (focus.firstStep < focus.step && focus.firstStep >= base) {
    void enqueue(async () => {
      try {
        await rewindTo(focus.firstStep);
      } catch (error) {
        showNotice(`Could not show the first violating step: ${errorMessage(error)}`, "warn");
      }
    });
  }
}

function violationKey(focus) {
  return `${focus.invariant}@${focus.firstStep}`;
}

function renderViolationCard(focus) {
  if (!focus || state.violationDismissed === violationKey(focus)) {
    el.violationCard.hidden = true;
    el.violationCard.replaceChildren();
    state.violationSignature = "";
    return;
  }
  const signature = `${violationKey(focus)}|${focus.step}|${focus.detail}|${state.timeline.furthest}`;
  el.violationCard.hidden = false;
  if (state.violationSignature === signature) {
    return;
  }
  state.violationSignature = signature;

  const head = document.createElement("div");
  head.className = "violation-head";
  const title = document.createElement("h2");
  title.id = "violation-title";
  title.textContent = `Invariant violated: ${focus.invariant}`;
  const close = document.createElement("button");
  close.type = "button";
  close.className = "ghost icon-button";
  close.dataset.testid = "violation-close";
  close.setAttribute("aria-label", "Close the violation summary");
  close.title = "Close";
  close.textContent = "✕";
  close.addEventListener("click", () => {
    state.violationDismissed = violationKey(focus);
    renderViolationCard(focus);
  });
  head.append(title, close);

  const detail = document.createElement("p");
  detail.className = "violation-detail";
  detail.dataset.testid = "violation-detail";
  detail.textContent = focus.detail || "(the checker gave no detail)";

  const where = document.createElement("p");
  where.className = "violation-meta";
  where.dataset.testid = "violation-meta";
  const at =
    focus.firstStep === focus.step
      ? `First reported at step ${focus.firstStep}; this is that step.`
      : `First reported at step ${focus.firstStep}; showing step ${focus.step}.`;
  const nodes = focus.nodes.length
    ? ` Highlighted on the graph: ${focus.nodes.join(", ")} (named in the detail).`
    : " The detail names no specific node.";
  where.textContent = `${at}${nodes} Run is paused.`;

  const actions = document.createElement("div");
  actions.className = "button-row";
  const goFirst = document.createElement("button");
  goFirst.type = "button";
  goFirst.dataset.testid = "violation-go-first";
  goFirst.textContent = `Go to step ${focus.firstStep}`;
  goFirst.disabled = focus.firstStep === focus.step || focus.firstStep < state.timeline.base;
  goFirst.addEventListener("click", () =>
    void runAction(() => rewindTo(focus.firstStep), { keepTimeline: true }),
  );
  const copy = document.createElement("button");
  copy.type = "button";
  copy.className = "primary";
  copy.dataset.testid = "violation-copy";
  copy.textContent = "Copy reproducer";
  copy.addEventListener("click", () => void runAction(copyReproducer, { keepTimeline: true }));
  const download = document.createElement("button");
  download.type = "button";
  download.dataset.testid = "violation-download";
  download.textContent = "Download script";
  download.addEventListener("click", () => void runAction(downloadScript, { keepTimeline: true }));
  actions.append(copy, download, goFirst);

  const note = document.createElement("p");
  note.className = "model-note";
  note.textContent =
    "Reported by the actual invariant checker for this seeded run. The reproducer rebuilds exactly this run; Download script replays it with vopr.";
  el.violationCard.replaceChildren(head, detail, where, actions, note);
}

// ---------------------------------------------------------------------------
// Node popover: the node's engine fields and its actions, next to the node.
// ---------------------------------------------------------------------------
function buildNodePopover() {
  const head = document.createElement("div");
  head.className = "popover-head";
  const title = document.createElement("h3");
  title.id = "node-popover-title";
  const status = document.createElement("span");
  status.className = "popover-status";
  const close = document.createElement("button");
  close.type = "button";
  close.className = "ghost icon-button";
  close.dataset.testid = "node-popover-close";
  close.setAttribute("aria-label", "Close node details");
  close.title = "Close";
  close.textContent = "✕";
  close.addEventListener("click", () => closeNodePopover());
  head.append(title, status, close);
  const rows = document.createElement("dl");
  rows.className = "popover-rows";
  const actions = document.createElement("div");
  actions.className = "button-row";
  const buttons = {};
  for (const [kind, label] of [
    ["crash", "Crash"],
    ["isolate", "Isolate"],
    ["rejoin", "Rejoin"],
    ["disable", "Disable"],
  ]) {
    const button = nodeButton(label, () => {
      if (state.popoverNode) {
        void runNodeAction(state.popoverNode, kind);
      }
    });
    button.dataset.testid = `node-popover-${kind}`;
    buttons[kind] = button;
    actions.append(button);
  }
  el.nodePopover.append(head, rows, actions);
  el.nodePopover.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      closeNodePopover();
    }
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && state.popoverNode) {
      closeNodePopover();
    }
  });
  // Pressing the empty canvas (a pan) closes it; pressing a node re-targets it.
  el.graph.addEventListener("pointerdown", (event) => {
    if (state.popoverNode && !event.target.closest(".vertex.node")) {
      closeNodePopover({ restoreFocus: false });
    }
  });
  state.popover = { title, status, rows, buttons };
}

function openNodePopover(id) {
  state.popoverNode = id;
  renderNodePopover();
  state.popover.buttons.crash.focus({ preventScroll: true });
}

function closeNodePopover({ restoreFocus = true } = {}) {
  const id = state.popoverNode;
  state.popoverNode = null;
  if (!el.nodePopover) {
    return;
  }
  el.nodePopover.hidden = true;
  if (restoreFocus && id) {
    const vertex = [...el.graph.querySelectorAll(".vertex.node")].find(
      (group) => group.querySelector(".v-id")?.textContent === id,
    );
    vertex?.focus({ preventScroll: true });
  }
}

function renderNodePopover() {
  if (!state.popoverNode || !state.popover) {
    return;
  }
  const details = nodeDetails(state.snapshot, state.popoverNode);
  if (!details) {
    closeNodePopover({ restoreFocus: false });
    return;
  }
  const { title, status, rows, buttons } = state.popover;
  title.textContent = details.id;
  status.textContent = details.status;
  status.className = `popover-status role-${details.status}`;
  const items = [];
  for (const [label, value] of details.rows) {
    const term = document.createElement("dt");
    term.textContent = label;
    const desc = document.createElement("dd");
    desc.textContent = String(value);
    items.push(term, desc);
  }
  rows.replaceChildren(...items);
  buttons.crash.textContent = details.crashed ? "Restart" : "Crash";
  buttons.disable.textContent = details.disabled ? "Enable" : "Disable";
  el.nodePopover.hidden = false;
  positionNodePopover();
}

const POPOVER_GAP = 12;

function positionNodePopover() {
  if (!state.popoverNode || el.nodePopover.hidden) {
    return;
  }
  const point = vertexClientPoint(state.popoverNode);
  if (!point) {
    return;
  }
  const stage = el.graphStage.getBoundingClientRect();
  const width = el.nodePopover.offsetWidth;
  const height = el.nodePopover.offsetHeight;
  const x = point.x - stage.left;
  const y = point.y - stage.top;
  let left = x + point.radius + POPOVER_GAP;
  if (left + width > stage.width - 8) {
    left = x - point.radius - POPOVER_GAP - width;
  }
  left = Math.max(8, Math.min(left, stage.width - width - 8));
  const top = Math.max(8, Math.min(y - height / 2, Math.max(8, stage.height - height - 8)));
  el.nodePopover.style.left = `${Math.round(left)}px`;
  el.nodePopover.style.top = `${Math.round(top)}px`;
}

// ---------------------------------------------------------------------------
// First-visit introduction: three pointers, dismissed once, reopened with "?".
// ---------------------------------------------------------------------------
const INTRO_KEY = "hydracache-demo-intro-seen";

// The graph legend folds into one "Legend" button. Open by default on wide
// screens, folded on narrow ones; a viewer's own choice is remembered (a
// per-viewer convenience, so storage failures just fall back to the default).
const LEGEND_KEY = "hydracache-demo-legend";

// The sticky control bar's height, so the sticky graph sits just below it.
function trackControlBarHeight() {
  const bar = document.querySelector(".controls");
  if (!bar || typeof ResizeObserver !== "function") return;
  new ResizeObserver(() => {
    document.documentElement.style.setProperty("--controls-sticky", `${Math.ceil(bar.getBoundingClientRect().height)}px`);
  }).observe(bar);
}

function bindLegend() {
  const legend = document.querySelector(".graph-legend");
  if (!legend) return;
  let stored = null;
  try {
    stored = window.localStorage.getItem(LEGEND_KEY);
  } catch {
    stored = null;
  }
  legend.open = stored ? stored === "open" : window.matchMedia("(min-width: 901px)").matches;
  legend.addEventListener("toggle", () => {
    try {
      window.localStorage.setItem(LEGEND_KEY, legend.open ? "open" : "closed");
    } catch {
      // Storage blocked: the choice lasts for this page only.
    }
    refreshGraphFit();
  });
}

function bindIntro() {
  let seen = false;
  try {
    seen = window.localStorage.getItem(INTRO_KEY) === "1";
  } catch (_error) {
    seen = false;
  }
  el.intro.hidden = seen;
  el.showIntro.setAttribute("aria-expanded", seen ? "false" : "true");
  el.introDismiss.addEventListener("click", () => {
    el.intro.hidden = true;
    el.showIntro.setAttribute("aria-expanded", "false");
    try {
      window.localStorage.setItem(INTRO_KEY, "1");
    } catch (_error) {
      // Storage can be unavailable; the intro still closes for this visit.
    }
    el.showIntro.focus({ preventScroll: true });
  });
  el.showIntro.addEventListener("click", () => {
    el.intro.hidden = !el.intro.hidden;
    el.showIntro.setAttribute("aria-expanded", el.intro.hidden ? "false" : "true");
    if (!el.intro.hidden) {
      el.intro.scrollIntoView({ block: "nearest" });
    }
  });
  for (const button of el.intro.querySelectorAll("[data-spotlight]")) {
    button.addEventListener("click", () => spotlight(document.querySelector(button.dataset.spotlight)));
  }
}

function spotlight(target) {
  if (!target) {
    return;
  }
  const reduced = document.documentElement.dataset.reducedMotion === "true";
  const box = target.closest(".control-group") ?? target;
  box.scrollIntoView({ block: "center", behavior: reduced ? "auto" : "smooth" });
  box.classList.remove("spotlight");
  void box.offsetWidth;
  box.classList.add("spotlight");
  window.setTimeout(() => box.classList.remove("spotlight"), 2000);
}

// ---------------------------------------------------------------------------
// Side-panel tabs (WAI-ARIA tabs: arrows / Home / End move between them).
// ---------------------------------------------------------------------------
function bindTabs() {
  for (const tab of el.tabs) {
    tab.addEventListener("click", () => selectTab(tab.dataset.tab));
    tab.addEventListener("keydown", (event) => {
      const index = el.tabs.indexOf(tab);
      const moves = {
        ArrowRight: index + 1,
        ArrowLeft: index - 1,
        Home: 0,
        End: el.tabs.length - 1,
      };
      if (!(event.key in moves)) {
        return;
      }
      event.preventDefault();
      const next = el.tabs[(moves[event.key] + el.tabs.length) % el.tabs.length];
      selectTab(next.dataset.tab);
      next.focus();
    });
  }
}

// Presentation mode: the graph fills the screen with only the run controls and
// the timeline around it (html.presenting), in browser full screen when the
// browser allows it. Esc, P or the button leave it.
function presenting() {
  return document.documentElement.classList.contains("presenting");
}

function setPresentation(on) {
  document.documentElement.classList.toggle("presenting", on);
  el.present.setAttribute("aria-pressed", on ? "true" : "false");
  el.present.setAttribute("aria-label", on ? "Leave presentation mode" : "Presentation mode");
  if (on && document.fullscreenEnabled && !document.fullscreenElement) {
    document.documentElement
      .requestFullscreen?.()
      .then(() => {
        state.presentFullscreen = true;
      })
      .catch(() => {
        // Full screen refused (e.g. not from a user gesture): the page layout
        // still gives the graph the whole window.
      });
  } else if (!on && state.presentFullscreen && document.fullscreenElement) {
    state.presentFullscreen = false;
    document.exitFullscreen?.().catch(() => {});
  }
  refreshGraphFit();
  announce(on ? "Presentation mode" : "Left presentation mode");
}

// The Fit button shows whether the view follows the cluster. After a pan or a
// zoom it is released, telling the user why the graph no longer re-centres.
function renderFitState(on) {
  el.zoomFit.setAttribute("aria-pressed", on ? "true" : "false");
  el.zoomFit.title = on ? "Fit the cluster (following)" : "Fit the cluster: the view was moved; press to re-centre";
}

function selectTab(name) {
  const changed = state.activeTab !== name;
  state.activeTab = name;
  for (const tab of el.tabs) {
    const selected = tab.dataset.tab === name;
    tab.setAttribute("aria-selected", selected ? "true" : "false");
    tab.tabIndex = selected ? 0 : -1;
  }
  for (const panel of el.tabPanels) {
    panel.hidden = panel.dataset.tabpanel !== name;
  }
  // Panels in the newly shown tab were skipped while hidden: paint them now.
  if (changed && state.snapshot) {
    renderTabPanels(state.snapshot);
  }
}

// The panels that live inside side-panel tabs. Each paints only while its tab
// is shown; selectTab calls this again so a newly shown tab is current.
function renderTabPanels(snapshot) {
  paintPanel("verdict-panel", "verdict", verdictPanelSignature(snapshot), () => renderVerdictPanel(snapshot));
  paintPanel("signals", "cluster", JSON.stringify([(snapshot.in_flight || []).slice(0, 8), snapshot.over_budget]), () => renderSignals(snapshot));
  paintPanel("clients", "clients", JSON.stringify([snapshot.clients, snapshot.subscribers]), () => renderClients(snapshot));
  paintPanel("replication", "clients", JSON.stringify([snapshot.replication, snapshot.writes, snapshot.hints, snapshot.repair]), () =>
    renderReplication(snapshot),
  );
  paintPanel("consistency", "data", consistencySignature(snapshot), () => renderConsistency(snapshot));
  paintPanel("keys", "data", keysSignature(snapshot), () => renderKeys(snapshot));
}

// ---------------------------------------------------------------------------
// Paint budget. Auto-play re-renders every tick; a panel is rebuilt only when
// it is visible (its tab is the active one, or it lives outside the tabs) and
// the slice of the snapshot it shows has changed since it was last painted.
// A panel skipped while hidden is forgotten, so showing its tab repaints it
// from the current snapshot: nothing shown is ever older than the engine.
// ---------------------------------------------------------------------------
function paintPanel(name, tab, signature, paint) {
  if (tab !== null && tab !== state.activeTab) {
    state.painted.delete(name);
    return false;
  }
  if (state.painted.get(name) === signature) {
    return false;
  }
  paint();
  state.painted.set(name, signature);
  return true;
}

function verdictPanelSignature(snapshot) {
  return JSON.stringify([
    snapshot.verdict,
    snapshot.first_violation_step,
    snapshot.checked_invariants,
    snapshot.step,
  ]);
}

function consistencySignature(snapshot) {
  return JSON.stringify([
    snapshot.consistency,
    snapshot.consistency_scope,
    keyAgreement(snapshot),
    snapshot.progress?.committed_entries,
  ]);
}

function keysSignature(snapshot) {
  return JSON.stringify([
    keyAgreement(snapshot),
    (state.changes?.keys ?? []).map((change) => `${change.key}|${change.node}`),
  ]);
}

// Small state markers on the tabs, so a problem in a hidden tab is still seen.
function renderTabBadges(snapshot) {
  const badge = (name, text, kind, title) => {
    const tab = el.tabs.find((candidate) => candidate.dataset.tab === name);
    const node = tab?.querySelector(".tab-badge");
    if (!node) {
      return;
    }
    node.textContent = text;
    node.className = `tab-badge${kind ? ` ${kind}` : ""}`;
    node.hidden = !text;
    tab.title = title;
  };
  const violated = snapshot.verdict.status !== "holding";
  badge("verdict", violated ? "✕" : "✓", violated ? "bad" : "ok", violated ? "Verdict: violated" : "Verdict: holding");
  const agree = keyAgreement(snapshot).agree;
  badge("data", agree ? "" : "≠", agree ? "" : "warn", agree ? "Keys and consistency" : "Replicas differ on a sampled key");
  const people = (snapshot.clients || []).length + (snapshot.subscribers || []).length;
  badge("clients", people ? String(people) : "", "", "Clients and subscribers");
  const down = (snapshot.nodes || []).filter((node) => node.crashed || node.disabled).length;
  badge("cluster", down ? `${down}↓` : "", down ? "warn" : "", down ? `${down} node(s) down` : "Links, nodes, progress, signals");
}

// ---------------------------------------------------------------------------
// Mobile run dock: Play / Step and the current step stay reachable once the
// controls have scrolled away. It mirrors the main buttons; it adds no state.
// ---------------------------------------------------------------------------
let controlsInView = true;

function bindDock() {
  el.dockPlay.addEventListener("click", () => el.play.click());
  el.dockStep.addEventListener("click", () => el.step.click());
  if (typeof IntersectionObserver === "function") {
    new IntersectionObserver((entries) => {
      controlsInView = entries.some((entry) => entry.isIntersecting);
      syncDock();
    }).observe(el.controls);
  }
  window.matchMedia?.("(max-width: 900px)").addEventListener?.("change", syncDock);
}

function syncDock() {
  if (!el.dock) {
    return;
  }
  const narrow = typeof window.matchMedia === "function" && window.matchMedia("(max-width: 900px)").matches;
  el.dock.hidden = !narrow || controlsInView;
  document.documentElement.classList.toggle("dock-open", !el.dock.hidden);
  el.dockPlay.textContent = el.play.textContent;
  el.dockPlay.disabled = el.play.disabled;
  el.dockStep.disabled = el.step.disabled;
  const snapshot = state.snapshot;
  el.dockStepLabel.textContent = snapshot ? `step ${snapshot.step}` : "loading";
  const violated = snapshot && snapshot.verdict.status !== "holding";
  el.dockVerdict.className = `dock-verdict ${snapshot ? (violated ? "bad" : "ok") : "warn"}`;
  el.dockVerdict.textContent = snapshot ? (violated ? "✕ violated" : "✓ holds") : "…";
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts (insight.shortcutFor decides; this only dispatches).
// ---------------------------------------------------------------------------
function shortcutTarget(target) {
  if (!(target instanceof Element)) {
    return "page";
  }
  if (target.closest("dialog[open]")) {
    return "field";
  }
  if (target.isContentEditable || target.matches("textarea, select, input:not([type=range]):not([type=checkbox])")) {
    return "field";
  }
  if (target.closest("button, summary, a[href], input, [role=tab], [role=button], [tabindex]")) {
    return "control";
  }
  return "page";
}

function bindShortcuts() {
  el.showShortcuts.addEventListener("click", openShortcuts);
  el.shortcutsClose.addEventListener("click", () => el.shortcuts.close());
  el.shortcuts.addEventListener("click", (event) => {
    if (event.target === el.shortcuts) {
      el.shortcuts.close();
    }
  });
  document.addEventListener("keydown", (event) => {
    if (event.defaultPrevented) {
      return;
    }
    const command = shortcutFor(event, shortcutTarget(event.target));
    if (!command) {
      return;
    }
    event.preventDefault();
    runShortcut(command);
  });
}

function openShortcuts() {
  if (!el.shortcuts.open) {
    el.shortcuts.showModal();
  }
}

function runShortcut(command) {
  const step = state.snapshot?.step ?? 0;
  const scrub = (delta) => {
    if (el.timelineInput.disabled) {
      return;
    }
    void runAction(() => rewindTo(step + delta), { keepTimeline: true });
  };
  switch (command) {
    case "help":
      openShortcuts();
      break;
    case "toggle-play":
      if (!el.play.disabled) {
        togglePlay();
      }
      break;
    case "step":
      if (!el.step.disabled) {
        el.step.click();
      }
      break;
    case "fit":
      fitGraph();
      break;
    case "present":
      setPresentation(!presenting());
      break;
    case "back":
      scrub(-1);
      break;
    case "forward":
      scrub(1);
      break;
    case "back-10":
      scrub(-10);
      break;
    case "forward-10":
      scrub(10);
      break;
    default:
      break;
  }
}

// Polite announcements for changes that are otherwise only visual.
function announce(text) {
  el.announcer.textContent = "";
  window.requestAnimationFrame(() => {
    el.announcer.textContent = text;
  });
}

boot();
