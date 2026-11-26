// Force-directed cluster graph (presentation only: positions carry no engine
// meaning). The page passes in its SVG element and a few hooks; the graph
// never reads page state directly.
import { keyAgreement, nodeStatus } from "./insight.js";

const SVG_NS = "http://www.w3.org/2000/svg";

// host: { svg, getSnapshot(), getSelectedLink(), onSelectLink(link), onClientPress(client),
//         onNodePress(id), getNodeMarks() -> { changed: Set, implicated: Set }, onPaint() }
let host = null;
let seenNodes = new Set();

export function initGraph(svgElement, hooks) {
  host = { svg: svgElement, ...hooks };
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
  // Tidy layout: cluster nodes on a ring, writers and subscribers just
  // outside it next to the node they attach to.
  ringBase: 128,
  ringPerNode: 9,
  ringMax: 196,
  entityGap: 112,
  entitySpread: 0.42,
  ease: 0.2,
};

const graphSim = {
  pos: new Map(), // id -> { x, y, fixed, kind }
  // Tidy-layout slot per vertex, recomputed on every sync; and positions the
  // user dragged a vertex to, kept until "Tidy" clears them.
  targets: new Map(),
  pins: new Map(),
  vertices: [], // [{ id, kind }]
  edges: [], // [{ a, b, kind }]
  dom: null,
  topoKey: "",
  view: { tx: 0, ty: 0, scale: 1 },
  // While true the view eases toward a transform that fits the whole cluster;
  // a manual pan or zoom turns it off until "Fit" is pressed.
  autoFit: true,
  drag: null,
  raf: null,
  packets: [],
  // Flight length in steps per message id, kept while the message is in flight.
  flightTotals: new Map(),
  packetEpoch: 0,
  // Identity of the in-flight set last drawn, so a re-render of the same step
  // does not restart the packets mid-flight.
  packetKey: "",
  // Causal staging: what each node currently shows, and role changes waiting
  // for the message that causes them to arrive (see stageNodeVisual).
  nodeShown: new Map(),
  pendingNodes: new Map(),
  lastLeaderId: null,
  // Role overlays (leader baton, term labels) and live vote tallies.
  overlays: [],
  tallies: new Map(),
  // Nodes back from a crash or a disable that still lag the live majority.
  recovering: new Set(),
  // Drop counters last drawn per node pair, to burst when they rise.
  dropTotals: new Map(),
  // When a push's data-flow pulse reaches each vertex (performance.now()
  // time), and flashes waiting for that moment.
  pulseArrivals: new Map(),
  pendingFlashes: new Map(),
  pulses: [],
  interactionsBound: false,
};

export function resetGraph() {
  seenNodes = new Set();
  if (graphSim.raf !== null) {
    cancelAnimationFrame(graphSim.raf);
    graphSim.raf = null;
  }
  graphSim.pos.clear();
  graphSim.targets = new Map();
  graphSim.pins = new Map();
  graphSim.topoKey = "";
  graphSim.view = { tx: 0, ty: 0, scale: 1 };
  setAutoFit(true);
  graphSim.drag = null;
  graphSim.packets = [];
  graphSim.flightTotals = new Map();
  graphSim.packetKey = "";
  graphSim.nodeShown = new Map();
  graphSim.pendingNodes = new Map();
  graphSim.lastLeaderId = null;
  graphSim.overlays = [];
  graphSim.tallies = new Map();
  graphSim.recovering = new Set();
  graphSim.dropTotals = new Map();
  graphSim.pulseArrivals = new Map();
  graphSim.pendingFlashes = new Map();
}

const VIEW_BOX = { width: 800, height: 520 };
const ZOOM_LIMITS = { min: 0.45, max: 2.4 };
const FIT_LIMITS = { min: 0.6, max: 1.35, maxCompact: 1.8 };
// Room kept free for the overlays: the hint on top, the legend at the bottom,
// the zoom buttons on the right. Narrow layouts move the legend below the
// canvas, so they only keep a small margin (and may zoom in further).
const FIT_INSETS = { top: 40, right: 56, bottom: 96, left: 28 };
const FIT_INSETS_COMPACT = { top: 20, right: 56, bottom: 20, left: 20 };

function overlaysOnCanvas() {
  const legend = host?.svg.parentElement?.querySelector(".graph-legend");
  return !legend || getComputedStyle(legend).position === "absolute";
}

// Keep the cluster clear of the legend overlay: measure where the legend's top
// edge falls in viewBox units (the SVG letterboxes the 800x520 viewBox), so a
// tall legend or a short canvas still leaves the nodes uncovered. A folded
// legend is one small button.
function fitInsets(view) {
  if (!overlaysOnCanvas()) return FIT_INSETS_COMPACT;
  const legend = host?.svg.parentElement?.querySelector(".graph-legend");
  if (!legend || !view.frame) return FIT_INSETS;
  const box = legend.getBoundingClientRect();
  if (box.height === 0) return FIT_INSETS;
  const below = (view.frame.bottom - box.top) / view.k + 10;
  return { ...FIT_INSETS, bottom: Math.max(36, Math.min(view.height * 0.45, below)) };
}

// The part of the canvas that is actually visible, in viewBox units. The SVG
// letterboxes its 800x520 viewBox: a tall canvas (a phone in presentation
// mode) shows more above and below it, a wide one more to the sides, and the
// fit uses all of that room.
function visibleViewBox() {
  const frame = host?.svg.getBoundingClientRect();
  if (!frame || frame.width === 0 || frame.height === 0) {
    return { x: 0, y: 0, width: VIEW_BOX.width, height: VIEW_BOX.height, frame: null, k: 1 };
  }
  const k = Math.min(frame.width / VIEW_BOX.width, frame.height / VIEW_BOX.height);
  const width = frame.width / k;
  const height = frame.height / k;
  return { x: (VIEW_BOX.width - width) / 2, y: (VIEW_BOX.height - height) / 2, width, height, frame, k };
}

/** Re-run the fit (e.g. after an overlay changed size) without forcing it on. */
export function refreshGraphFit() {
  startGraphSim();
}

// Every change of the fit mode goes through here so the app can show it (the
// Fit button reads "pressed" while the view follows the cluster).
function setAutoFit(value) {
  if (graphSim.autoFit === value) return;
  graphSim.autoFit = value;
  host?.onAutoFitChange?.(value);
}

/** Zoom around the canvas centre (graph view buttons). */
export function zoomGraph(factor) {
  setAutoFit(false);
  zoomAround({ x: VIEW_BOX.width / 2, y: VIEW_BOX.height / 2 }, factor);
}

/** Return to the automatic "fit the whole cluster" view. */
export function fitGraph() {
  setAutoFit(true);
  startGraphSim();
}

// Zoom keeping the point under `anchor` (viewBox coordinates) still.
function zoomAround(anchor, factor) {
  const view = graphSim.view;
  const before = { x: (anchor.x - view.tx) / view.scale, y: (anchor.y - view.ty) / view.scale };
  view.scale = Math.max(ZOOM_LIMITS.min, Math.min(ZOOM_LIMITS.max, view.scale * factor));
  view.tx = anchor.x - before.x * view.scale;
  view.ty = anchor.y - before.y * view.scale;
  paintGraph(performance.now());
}

// The transform that fits every vertex (plus its radius and labels) into the
// viewBox with a margin, clamped so a small cluster is not blown up.
function fitTarget() {
  if (graphSim.pos.size === 0) {
    return null;
  }
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const p of graphSim.pos.values()) {
    const r = p.kind === "node" ? GRAPH.nodeRadius + 14 : GRAPH.entityRadius + 40;
    minX = Math.min(minX, p.x - r);
    minY = Math.min(minY, p.y - r);
    maxX = Math.max(maxX, p.x + r);
    maxY = Math.max(maxY, p.y + r);
  }
  const overlays = overlaysOnCanvas();
  const view = visibleViewBox();
  const insets = fitInsets(view);
  const maxScale = overlays ? FIT_LIMITS.max : FIT_LIMITS.maxCompact;
  const box = {
    x: view.x + insets.left,
    y: view.y + insets.top,
    width: view.width - insets.left - insets.right,
    height: view.height - insets.top - insets.bottom,
  };
  const width = Math.max(maxX - minX, 1);
  const height = Math.max(maxY - minY, 1);
  const scale = Math.max(FIT_LIMITS.min, Math.min(maxScale, box.width / width, box.height / height));
  return {
    scale,
    tx: box.x + box.width / 2 - ((minX + maxX) / 2) * scale,
    ty: box.y + box.height / 2 - ((minY + maxY) / 2) * scale,
  };
}

// Ease the view one frame toward the fit target; returns the remaining error
// so the animation loop keeps running until the view has settled.
function easeTowardFit() {
  if (!graphSim.autoFit || graphSim.drag) {
    return 0;
  }
  const target = fitTarget();
  if (!target) {
    return 0;
  }
  const view = graphSim.view;
  const reduced = document.documentElement.dataset.reducedMotion === "true";
  const k = reduced ? 1 : 0.18;
  view.scale += (target.scale - view.scale) * k;
  view.tx += (target.tx - view.tx) * k;
  view.ty += (target.ty - view.ty) * k;
  return Math.abs(target.scale - view.scale) * 100 + Math.abs(target.tx - view.tx) + Math.abs(target.ty - view.ty);
}

export function syncGraph(snapshot) {
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
  // Backlog per node pair: messages that will not be delivered on the next
  // step (normal traffic is always delivered next step; delays pile up).
  const stepMillis = snapshot.step > 0 ? Math.max(1, Number(snapshot.logical_time_millis) / snapshot.step) : 1;
  const backlogs = new Map();
  for (const message of snapshot.in_flight || []) {
    if ((Number(message.remaining_millis) || 0) <= stepMillis) continue;
    const pair = [message.from, message.to].sort().join("|");
    backlogs.set(pair, (backlogs.get(pair) ?? 0) + 1);
  }
  for (const link of snapshot.links || []) {
    const key = [link.from, link.to].slice().sort().join("|");
    if (drawnPairs.has(key)) continue;
    drawnPairs.add(key);
    const reverse = (snapshot.links || []).find((o) => o.from === link.to && o.to === link.from);
    const delay = Math.max(Number(link.delay_millis) || 0, Number(reverse?.delay_millis) || 0);
    const inFlight = (Number(link.in_flight) || 0) + (Number(reverse?.in_flight) || 0);
    const backlog = backlogs.get(key) ?? 0;
    const drops = pairDrops(link, reverse);
    edges.push({ a: link.from, b: link.to, kind: "node", state: worstLinkState(link.state, reverse?.state), delay, inFlight, backlog, drops, from: link.from, to: link.to });
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
  graphSim.targets = tidyTargets(nodes, clients, subscribers);
  for (const id of [...graphSim.pins.keys()]) if (!liveIds.has(id)) graphSim.pins.delete(id);
  vertices.forEach((v) => ensureVertexPosition(v));

  // Rebuild DOM only when the set of vertices/edges changes.
  const topoKey = vertices.map((v) => v.id).join(",") + "::" + edges.map((e) => `${e.a}>${e.b}`).join(",");
  if (topoKey !== graphSim.topoKey) {
    buildGraphDom(snapshot);
    graphSim.topoKey = topoKey;
  }

  // Packets first: node role changes are timed to their arrival.
  rebuildPackets(snapshot);
  updateGraphVisuals(snapshot);
  syncVoteTally(snapshot);
  startGraphSim();
}

// The tidy layout: the same snapshot always gives the same picture. Nodes sit
// on a ring in id order (starting at the top); each writer and subscriber sits
// outside the ring, beside the node it is attached to, fanned out when several
// share a node.
export function tidyTargets(nodes, clients = [], subscribers = []) {
  const targets = new Map();
  const ids = nodes.map((node) => node.id).sort((a, b) => a.localeCompare(b, "en", { numeric: true }));
  const radius = Math.min(GRAPH.ringMax, GRAPH.ringBase + GRAPH.ringPerNode * ids.length);
  const angles = new Map();
  ids.forEach((id, index) => {
    const angle = -Math.PI / 2 + (index * Math.PI * 2) / Math.max(ids.length, 1);
    angles.set(id, angle);
    targets.set(id, {
      x: GRAPH.center.x + Math.cos(angle) * (ids.length > 1 ? radius : 0),
      y: GRAPH.center.y + Math.sin(angle) * (ids.length > 1 ? radius : 0),
    });
  });
  const entities = [
    ...clients.map((client) => ({ id: `client:${client.id}`, node: client.connected_node })),
    ...subscribers.map((subscriber) => ({ id: `sub:${subscriber.id}`, node: subscriber.connected_node })),
  ];
  const byNode = new Map();
  for (const entity of entities) {
    const list = byNode.get(entity.node) ?? [];
    list.push(entity);
    byNode.set(entity.node, list);
  }
  for (const [node, list] of byNode) {
    // Unattached entities sit below the cluster.
    const base = angles.get(node) ?? Math.PI / 2;
    const distance = (angles.has(node) && ids.length > 1 ? radius : 0) + GRAPH.entityGap;
    list.forEach((entity, index) => {
      const angle = base + (index - (list.length - 1) / 2) * GRAPH.entitySpread;
      targets.set(entity.id, {
        x: GRAPH.center.x + Math.cos(angle) * distance,
        y: GRAPH.center.y + Math.sin(angle) * distance,
      });
    });
  }
  return targets;
}

function ensureVertexPosition(vertex) {
  if (graphSim.pos.has(vertex.id)) {
    graphSim.pos.get(vertex.id).kind = vertex.kind;
    return;
  }
  // A new vertex starts at its slot (an entity at its node, so it slides out).
  const slot = graphSim.targets.get(vertex.id) ?? GRAPH.center;
  const anchor = vertex.kind === "node" ? slot : graphSim.pos.get(vertex.ref?.connected_node) ?? slot;
  graphSim.pos.set(vertex.id, { x: anchor.x, y: anchor.y, fixed: false, kind: vertex.kind });
}

/** Clear dragged positions and return every vertex to the tidy layout. */
export function tidyGraph() {
  graphSim.pins.clear();
  startGraphSim();
}

function buildGraphDom(snapshot) {
  host.svg.replaceChildren();
  const viewport = svg("g", { class: "graph-viewport" });
  const edgeLayer = svg("g", { class: "links" });
  const entityLinkLayer = svg("g", { class: "entity-links", "aria-hidden": "true" });
  // Writer -> subscriber of the same namespace: where its events will go.
  const flowLayer = svg("g", { class: "namespace-flows", "aria-hidden": "true" });
  const packetLayer = svg("g", { class: "packet-layer", "aria-hidden": "true" });
  const pulseLayer = svg("g", { class: "pulse-layer", "aria-hidden": "true" });
  const labelLayer = svg("g", { class: "link-labels", "aria-hidden": "true" });
  const vertexLayer = svg("g", { class: "vertices" });
  // Above the vertices: leader baton, term labels and vote tallies. Hidden from
  // assistive tech; the node's own name carries its role and term.
  const roleLayer = svg("g", { class: "role-layer", "aria-hidden": "true" });
  viewport.append(edgeLayer, flowLayer, entityLinkLayer, packetLayer, pulseLayer, labelLayer, vertexLayer, roleLayer);
  host.svg.append(viewport);

  const dom = { viewport, packetLayer, pulseLayer, roleLayer, flowLayer, flows: [], edges: [], entityEdges: [], vertices: new Map() };
  // A rebuilt canvas has no packets or overlays drawn yet.
  graphSim.packetKey = "";
  graphSim.overlays = [];
  graphSim.tallies = new Map();

  for (const edge of graphSim.edges) {
    if (edge.kind === "node") {
      const visual = svg("line", { class: "link" });
      // A pill behind the text keeps the label readable over lines and packets.
      const label = svg("g", { class: "link-label" });
      label.append(svg("rect", { class: "link-pill", rx: 9 }), svg("text", { class: "link-text", y: 4 }));
      // The wide invisible hit line is also the keyboard target for the link.
      const hit = svg("line", { class: "link-hit", tabindex: 0, role: "button" });
      // A partitioned link is drawn broken: a gap with two bars across it.
      const cut = svg("g", { class: "link-cut", "aria-hidden": "true" });
      cut.append(
        svg("line", { class: "link-cut-gap", x1: -9, y1: 0, x2: 9, y2: 0 }),
        svg("line", { class: "link-cut-bar", x1: -6, y1: -8, x2: -6, y2: 8 }),
        svg("line", { class: "link-cut-bar", x1: 6, y1: -8, x2: 6, y2: 8 }),
      );
      hit.addEventListener("click", (event) => {
        event.stopPropagation();
        host.onSelectLink({ from: edge.from, to: edge.to });
      });
      onActivateKey(hit, () => host.onSelectLink({ from: edge.from, to: edge.to }));
      edgeLayer.append(visual, cut, hit);
      labelLayer.append(label);
      dom.edges.push({ edge, visual, hit, label, cut });
    } else {
      const line = svg("line", { class: "entity-link" });
      entityLinkLayer.append(line);
      dom.entityEdges.push({ edge, line });
    }
  }

  for (const vertex of graphSim.vertices) {
    const group = svg("g", { class: "vertex" });
    if (vertex.kind === "node") {
      const badge = svg("g", {
        class: "v-badge",
        transform: `translate(${GRAPH.nodeRadius * 0.72} ${-GRAPH.nodeRadius * 0.72})`,
        "aria-hidden": "true",
      });
      // Glyph for most roles; a drawn icon where fonts are unreliable.
      badge.append(svg("circle", { r: 13 }), svg("text", { y: 5 }), svg("path", { class: "v-icon" }));
      // Commit progress against the furthest node (the leader in steady state).
      const progress = svg("g", { class: "v-progress", transform: "translate(-20 27)", "aria-hidden": "true" });
      progress.append(
        svg("rect", { class: "v-progress-track", width: 40, height: 4, rx: 2 }),
        // Writes already on their way to this node, then writes applied here.
        svg("rect", { class: "v-progress-pending", width: 0, height: 4, rx: 2 }),
        svg("rect", { class: "v-progress-fill", width: 0, height: 4, rx: 2 }),
      );
      group.append(
        // Pulses around a campaigning candidate (CSS shows it for that role).
        svg("circle", { class: "v-ring", r: GRAPH.nodeRadius + 9 }),
        svg("circle", { class: "v-body", r: GRAPH.nodeRadius }),
        svg("text", { class: "v-id", y: -5 }),
        svg("text", { class: "v-sub", y: 17 }),
        progress,
        badge,
      );
    } else {
      group.append(
        svg("circle", { r: GRAPH.entityRadius }),
        // Role glyph inside (writer sends up, subscriber receives down); the
        // name and namespace sit below, where they fit.
        svg("text", { class: "entity-glyph", y: 6, "aria-hidden": "true" }, vertex.kind === "client" ? "↑" : "↓"),
        svg("text", { class: "entity-id", y: 36 }),
        svg("text", { class: "entity-sub", y: 50 }),
      );
      group.setAttribute("tabindex", "0");
      group.setAttribute("role", "button");
      if (vertex.kind === "client") {
        group.append(svg("title", {}, "Click or press Enter to push (drag to move)"));
        onActivateKey(group, () => host.onClientPress(vertex.ref));
      } else {
        // A subscriber opens its row in the Subscribers list.
        group.append(svg("title", {}, "Click or press Enter to show this subscriber (drag to move)"));
        onActivateKey(group, () => host.onSubscriberPress?.(vertex.ref));
      }
    }
    if (vertex.kind === "node") {
      // A press (not a drag) or Enter opens the node's details and actions.
      group.setAttribute("tabindex", "0");
      group.setAttribute("role", "button");
      group.setAttribute("aria-haspopup", "dialog");
      onActivateKey(group, () => host.onNodePress?.(vertex.id));
    }
    bindVertexDrag(group, vertex);
    vertexLayer.append(group);
    dom.vertices.set(vertex.id, { group, vertex });
  }

  graphSim.dom = dom;
}

// ---------------------------------------------------------------------------
// Causal staging. A role change that a message causes is shown when that
// message arrives: a voter turns when the vote request lands, a follower when
// the leader's heartbeat lands. Changes with a local cause (a candidate's own
// timeout, a crash, a user action) and changes with no arriving cause show at
// once. Reduced motion shows everything at once.
// ---------------------------------------------------------------------------
const CAUSING_KINDS = new Set(["vote_request", "leader_heartbeat"]);

function packetArrivalTime(packet) {
  const period = Math.max(300, Number(host.getStepMillis?.()) || 1150);
  return graphSim.packetEpoch + period * (Math.min(packet.lane, 3) * 0.12 + 0.6);
}

function causeArrival(nodeId) {
  let latest = null;
  for (const packet of graphSim.packets) {
    if (packet.to !== nodeId || packet.doomed || packet.end < 1 || !CAUSING_KINDS.has(packet.kind)) continue;
    const at = packetArrivalTime(packet);
    latest = latest === null ? at : Math.max(latest, at);
  }
  return latest;
}

function stageNodeVisual(item, node, marks, snapshot) {
  const shown = graphSim.nodeShown.get(node.id);
  const status = nodeStatus(node);
  const at = shown && shown.status !== status && !reducedMotion() ? causeArrival(node.id) : null;
  if (at === null) {
    applyNodeVisual(item, node, marks, snapshot);
    return;
  }
  graphSim.pendingNodes.set(node.id, { at, apply: () => applyNodeVisual(item, node, marks, snapshot) });
}

// A node that goes down gets a short hit (it greys out and its badge grows in);
// a node that comes back is marked "recovering" while its replicas lag the
// live majority, and flashes "synced" once they agree again.
function markCrashAndRecovery(group, node, status, shown, behind) {
  const down = status === "crashed" || status === "disabled";
  const wasDown = shown && (shown.status === "crashed" || shown.status === "disabled");
  if (down && shown && !wasDown && !reducedMotion()) {
    restartClass(group, "down-hit");
  }
  if (down) {
    graphSim.recovering.delete(node.id);
    return;
  }
  if (wasDown) graphSim.recovering.add(node.id);
  if (!graphSim.recovering.has(node.id)) return;
  if (behind) {
    group.classList.add("recovering");
    return;
  }
  graphSim.recovering.delete(node.id);
  if (!reducedMotion()) restartClass(group, "synced");
}

// A change a push caused shows when the push's message reaches the vertex
// (the data-flow pulse), not when the engine already holds it: the entry node
// flashes first, then each replica, then the subscribers. Without a pulse in
// flight (reduced motion, or a change of another cause) it flashes at once.
function flashWhenReached(id, group, name) {
  const at = graphSim.pulseArrivals.get(id);
  // The push's copy for this node is pending or lost: nothing arrived, so
  // nothing flashes (and nothing waits for it).
  if (at === Number.POSITIVE_INFINITY) {
    graphSim.pulseArrivals.delete(id);
    return;
  }
  if (at !== undefined && at > performance.now()) {
    graphSim.pendingFlashes.set(id, { at, group, name });
    return;
  }
  restartClass(group, name);
}

function applyDueFlashes(now) {
  for (const [id, flash] of graphSim.pendingFlashes) {
    if (now < flash.at) continue;
    graphSim.pendingFlashes.delete(id);
    graphSim.pulseArrivals.delete(id);
    // The vertex may have been rebuilt since; flash the current one.
    restartClass(graphSim.dom?.vertices.get(id)?.group ?? flash.group, flash.name);
  }
}

function restartClass(element, name) {
  element.classList.remove(name);
  void element.getBoundingClientRect();
  element.classList.add(name);
}

function applyDueNodes(now, all = false) {
  for (const [id, pending] of graphSim.pendingNodes) {
    if (all || now >= pending.at) {
      graphSim.pendingNodes.delete(id);
      pending.apply();
    }
  }
}

function reducedMotion() {
  return document.documentElement.dataset.reducedMotion === "true";
}

// A node's write progress: writes it applied, writes in flight to it, and the
// reference it is measured against: the most up-to-date live node. (Writes do
// not always enter through the leader, and every write reaches every replica,
// so the furthest node is the target.)
// Older snapshots without applied_writes fall back to replica agreement.
export function nodeProgress(snapshot, node) {
  if (node.applied_writes === undefined) {
    const { agreeing, sampled } = replicaAgreement(snapshot, node.id);
    return { applied: agreeing, pending: 0, reference: sampled, missing: sampled - agreeing };
  }
  const live = (snapshot.nodes || []).filter((candidate) => !candidate.crashed && !candidate.disabled);
  const reference = Math.max(0, node.applied_writes, ...live.map((candidate) => candidate.applied_writes ?? 0));
  // Only writes that will actually be delivered count as arriving: none to a
  // down node, none over a cut link (the network drops those on delivery).
  const cut = new Set(
    (snapshot.links || []).filter((link) => link.state === "partitioned").map((link) => `${link.from}>${link.to}`),
  );
  const pending =
    node.crashed || node.disabled
      ? 0
      : (snapshot.in_flight || []).filter(
          (message) =>
            message.to === node.id &&
            String(message.kind).startsWith("replicate_") &&
            !cut.has(`${message.from}>${message.to}`),
        ).length;
  const applied = node.applied_writes;
  return { applied, pending, reference, missing: Math.max(0, reference - applied - pending) };
}

// Per-node replica agreement, computed once per snapshot.
let agreementCache = { snapshot: null, byNode: new Map() };

function replicaAgreement(snapshot, nodeId) {
  if (agreementCache.snapshot !== snapshot) {
    const byNode = new Map();
    for (const key of keyAgreement(snapshot).keys) {
      for (const cell of key.cells) {
        if (cell.version === null) continue;
        const entry = byNode.get(cell.node) ?? { agreeing: 0, sampled: 0 };
        entry.sampled += 1;
        if (cell.version === key.majority) entry.agreeing += 1;
        byNode.set(cell.node, entry);
      }
    }
    agreementCache = { snapshot, byNode };
  }
  return agreementCache.byNode.get(nodeId) ?? { agreeing: 0, sampled: 0 };
}

// What a node looks like for one snapshot: role class, labels and name.
function applyNodeVisual(item, node, marks, snapshot) {
  const status = nodeStatus(node);
  const shown = graphSim.nodeShown.get(node.id);
  const isNew = !seenNodes.has(node.id);
  seenNodes.add(node.id);
  item.group.setAttribute(
    "class",
    [
      "vertex",
      "node",
      `role-${status}`,
      node.crashed ? "crashed" : "",
      node.disabled ? "disabled" : "",
      isNew ? "joining" : "",
      marks.implicated.has(node.id) ? "implicated" : "",
    ]
      .filter(Boolean)
      .join(" "),
  );
  // Flash for what matters on the graph: a role, term or membership change,
  // or a write a push carried here. Routine commit progress from the
  // background workload shows on the agreement bar, not as a flash.
  const notable = marks.notable ? marks.notable.has(node.id) : marks.changed.has(node.id);
  if (notable || (marks.changed.has(node.id) && graphSim.pulseArrivals.has(node.id))) {
    flashWhenReached(node.id, item.group, "changed");
  }
  if (graphSim.tallies.has(node.id)) item.group.classList.add("campaigning");
  // A re-render must not reveal the badge while the baton is still flying.
  if (graphSim.overlays.some((overlay) => overlay.group === item.group)) {
    item.group.classList.add("baton-incoming");
  }
  // The trailing space is not drawn (SVG drops it) but keeps the id and the
  // status apart in the vertex's text, so its accessible name, which starts
  // with "id, status", matches the visible text (WCAG 2.5.3).
  item.group.querySelector(".v-id").textContent = `${node.id} `;
  item.group.querySelector(".v-sub").textContent = status;
  const icon = BADGE_ICONS[status];
  item.group.querySelector(".v-badge text").textContent = icon ? "" : ROLE_BADGES[status] ?? "";
  item.group.querySelector(".v-icon").setAttribute("d", icon ?? "");
  // The bar: writes this node applied against the most up-to-date live node
  // (schema 9 applied_writes), with writes already in flight to it drawn
  // lighter, so a node one replication step behind is "arriving", not
  // "behind".
  // Replica agreement stays a second signal: a dashed red track when the
  // node's stored values differ from the live majority.
  const progress = nodeProgress(snapshot, node);
  const { agreeing, sampled } = replicaAgreement(snapshot, node.id);
  // While writes are still replicating, the writer briefly holds a value the
  // majority has not received yet; that is normal, so the red track waits
  // until replication settles (or the node is genuinely behind).
  const replicating = (snapshot.in_flight || []).some((message) => String(message.kind).startsWith("replicate_"));
  const differs = sampled > 0 && agreeing < sampled && (!replicating || progress.missing > 0);
  const bar = item.group.querySelector(".v-progress");
  const scale = progress.reference > 0 ? 40 / progress.reference : 0;
  bar.classList.toggle("empty", progress.reference === 0);
  bar.classList.toggle("behind", progress.missing > 0);
  bar.classList.toggle("diverged", differs);
  bar.querySelector(".v-progress-fill").setAttribute("width", Math.min(40, progress.applied * scale).toFixed(1));
  bar.querySelector(".v-progress-pending").setAttribute(
    "width",
    Math.min(40, (progress.applied + progress.pending) * scale).toFixed(1),
  );
  bar.dataset.applied = `${progress.applied}/${progress.reference}`;
  bar.dataset.pending = String(progress.pending);
  bar.dataset.agree = `${agreeing}/${sampled}`;
  item.group.setAttribute("aria-label", `${node.id}, ${status}, term ${node.term}`);
  markCrashAndRecovery(item.group, node, status, shown, progress.missing > 0 || differs);
  graphSim.nodeShown.set(node.id, { status, term: node.term });
  announceRoleChange(node, status, shown, snapshot);
}

function updateGraphVisuals(snapshot) {
  // A newer snapshot supersedes anything still waiting from the last one.
  applyDueNodes(0, true);
  const marks = host.getNodeMarks?.() ?? { changed: new Set(), implicated: new Set() };
  const nodeById = new Map((snapshot.nodes || []).map((n) => [n.id, n]));
  for (const [, item] of graphSim.dom.vertices) {
    const v = item.vertex;
    if (v.kind === "node") {
      const node = nodeById.get(v.id);
      if (!node) continue;
      stageNodeVisual(item, node, marks, snapshot);
    } else {
      // Vertices are rebuilt only on topology change, so read the live entity.
      const ref =
        (v.kind === "client" ? snapshot.clients : snapshot.subscribers)?.find((entity) => entity.id === v.ref.id) ??
        v.ref;
      v.ref = ref;
      const active = v.kind === "client" ? Boolean(ref.last_op) : Boolean(ref.last_event);
      item.group.setAttribute("class", ["vertex", "entity", v.kind, active ? "active" : "", "clickable"].filter(Boolean).join(" "));
      const name = v.kind === "client" ? ref.id : ref.client_id || ref.id;
      // Trailing space: not drawn, but keeps name and namespace apart so the
      // accessible name starts with the visible text (WCAG 2.5.3).
      item.group.querySelector(".entity-id").textContent = `${name} `;
      item.group.querySelector(".entity-sub").textContent = ref.namespace || "";
      item.group.setAttribute(
        "aria-label",
        v.kind === "client"
          ? `${name} ${ref.namespace}, writer; press Enter to push`
          : `${name} ${ref.namespace}, subscriber; press Enter to show it`,
      );
    }
  }
  // The DOM keeps the edge objects from when it was built (topology change
  // only); take state from the current edges so partitions and delays show.
  const liveEdges = new Map(graphSim.edges.map((edge) => [`${edge.a}>${edge.b}`, edge]));
  const current = host.getSelectedLink();
  for (const { edge: built, visual, hit, label, cut } of graphSim.dom.edges) {
    const edge = liveEdges.get(`${built.a}>${built.b}`) ?? built;
    const selected = current && current.from === edge.from && current.to === edge.to;
    visual.setAttribute("class", ["link", edge.state, selected ? "selected" : ""].filter(Boolean).join(" "));
    cut.classList.toggle("shown", edge.state === "partitioned");
    if (edge.drops) {
      burstNewDrops(edge);
      hit.dataset.dropped = String(edge.drops.total);
    }
    // A pair with held-back messages swells with its backlog.
    const backlog = edge.backlog ?? 0;
    visual.style.strokeWidth = backlog > 0 ? String(3 + Math.min(5, backlog * 1.5)) : "";
    visual.dataset.backlog = String(backlog);
    label.setAttribute("class", `link-label ${edge.state}`);
    setLinkLabel(label, linkLabelText(edge));
    const dropped = edge.drops?.total ? `, ${dropsText(edge.drops)}` : "";
    hit.setAttribute("aria-label", `link ${edge.from} to ${edge.to}, ${edge.state}${dropped}${selected ? ", selected" : ""}`);
    setHitTitle(hit, `${edge.from} ↔ ${edge.to}: ${edge.state}; ${dropsText(edge.drops)}`);
    hit.setAttribute("aria-pressed", selected ? "true" : "false");
  }
  syncNamespaceFlows(snapshot);
  for (const { edge: built, line } of graphSim.dom.entityEdges) {
    const edge = liveEdges.get(`${built.a}>${built.b}`) ?? built;
    line.setAttribute("class", ["entity-link", edge.kind === "entity" && edge.b.startsWith("sub:") ? "subscriber" : "client", edge.active ? "active" : ""].filter(Boolean).join(" "));
  }
}

function onActivateKey(target, activate) {
  target.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      activate();
    }
  });
}

// Packets follow the engine's delivery schedule instead of a decorative loop.
// A message's flight lasts `remaining_millis` (in steps) + 1 steps, measured the
// first time it is seen; on each step it covers that step's share of the edge,
// so an undelayed message crosses in one step and a delayed one crawls. While
// the run waits, the current step's share replays at the play speed. A message
// the engine will drop on delivery (its directed link is cut, or its target is
// down) is drawn as doomed and fades out before it arrives.
export function packetFlights(snapshot, previous = new Map()) {
  const stepMillis = snapshot.step > 0 ? Math.max(1, Number(snapshot.logical_time_millis) / snapshot.step) : 1;
  const nodes = new Map((snapshot.nodes || []).map((node) => [node.id, node]));
  const links = new Map((snapshot.links || []).map((link) => [`${link.from}>${link.to}`, link.state]));
  const lanes = new Map();
  const totals = new Map();
  const flights = (snapshot.in_flight || []).slice(0, 16).map((message) => {
    const left = Math.ceil(Math.max(0, Number(message.remaining_millis) || 0) / stepMillis);
    const total = Math.max(previous.get(message.id) ?? 0, left + 1);
    totals.set(message.id, total);
    const flown = total - left - 1;
    const target = nodes.get(message.to);
    const doomed =
      links.get(`${message.from}>${message.to}`) === "partitioned" ||
      Boolean(target && (target.crashed || target.disabled || target.up === false));
    const edge = [message.from, message.to].sort().join("|");
    const lane = lanes.get(edge) ?? 0;
    lanes.set(edge, lane + 1);
    return { message, start: flown / total, end: (flown + 1) / total, lane, doomed };
  });
  return { flights, totals };
}

function rebuildPackets(snapshot) {
  if (!graphSim.dom) return;
  const key = `${snapshot.step}|${snapshot.logical_time_millis}|${(snapshot.in_flight || []).map((message) => message.id).join(",")}`;
  if (key === graphSim.packetKey) return;
  graphSim.packetKey = key;
  graphSim.packets = [];
  graphSim.dom.packetLayer.replaceChildren();
  graphSim.packetEpoch = performance.now();
  const { flights, totals } = packetFlights(snapshot, graphSim.flightTotals);
  graphSim.flightTotals = totals;
  const { shown, folded } = foldRoutineTraffic(flights);
  graphSim.edgeBadges = [...folded].map(([pair, count]) => {
    const [a, b] = pair.split("|");
    const badge = svg("g", { class: "edge-badge" });
    badge.dataset.pair = pair;
    badge.append(svg("rect", { x: -15, y: -9, width: 30, height: 18, rx: 9 }), svg("text", { y: 4 }, `+${count}`));
    graphSim.dom.packetLayer.append(badge);
    return { a, b, badge };
  });
  for (const { message, start, end, lane, doomed } of shown) {
    if (!graphSim.pos.has(message.from) || !graphSim.pos.has(message.to)) continue;
    const family = packetFamily(message.kind);
    const kind = `${packetKind(message.kind)}${doomed ? " doomed" : ""}`;
    // Control traffic (heartbeats, acks) stays quiet: small and trail-less.
    const trail = family === "control" ? null : svg("line", { class: `packet-trail ${kind}` });
    const dot = svg("g", { class: `packet ${kind}` });
    dot.append(packetShape(family));
    const label = family === "data" && message.key ? svg("text", { class: "packet-key", y: -13 }, shortKey(message.key)) : null;
    if (label) dot.append(label);
    Object.assign(dot.dataset, {
      from: message.from,
      to: message.to,
      kind: message.kind,
      family,
      span: `${start.toFixed(3)}-${end.toFixed(3)}`,
    });
    if (trail) graphSim.dom.packetLayer.append(trail);
    graphSim.dom.packetLayer.append(dot);
    graphSim.packets.push({ dot, trail, label, family, from: message.from, to: message.to, kind: message.kind, start, end, lane, doomed });
  }
}

// A busy edge folds its routine traffic: heartbeats and acks that cross in
// one step and arrive keep one dot per direction, the rest become a "+N"
// badge. Votes, data, events, delayed and doomed messages always show.
export function foldRoutineTraffic(flights) {
  const routine = (flight) =>
    packetFamily(flight.message.kind) === "control" && flight.start === 0 && flight.end === 1 && !flight.doomed;
  const perPair = new Map();
  for (const flight of flights) {
    const pair = [flight.message.from, flight.message.to].sort().join("|");
    perPair.set(pair, (perPair.get(pair) ?? 0) + 1);
  }
  const keptDirections = new Set();
  const folded = new Map();
  const shown = [];
  for (const flight of flights) {
    const pair = [flight.message.from, flight.message.to].sort().join("|");
    const direction = `${flight.message.from}>${flight.message.to}`;
    if (perPair.get(pair) > 3 && routine(flight)) {
      if (keptDirections.has(direction)) {
        folded.set(pair, (folded.get(pair) ?? 0) + 1);
        continue;
      }
      keptDirections.add(direction);
    }
    shown.push(flight);
  }
  return { shown, folded };
}

// Shape per message family, so kinds read without relying on colour.
function packetShape(family) {
  switch (family) {
    case "vote":
      return svg("path", { class: "packet-shape", d: "M0 -10 L10 0 L0 10 L-10 0 Z" });
    case "data":
      return svg("rect", { class: "packet-shape", x: -8, y: -8, width: 16, height: 16, rx: 3 });
    case "event":
      return svg("path", { class: "packet-shape", d: "M0 -10 L9.5 7 L-9.5 7 Z" });
    case "control":
      return svg("circle", { class: "packet-shape", r: 5 });
    default:
      return svg("circle", { class: "packet-shape", r: 8 });
  }
}

function shortKey(key) {
  const text = String(key);
  return text.length > 12 ? `${text.slice(0, 11)}…` : text;
}

// Ease every vertex toward its place: a dragged position if the user moved
// it, otherwise its tidy slot. Returns the remaining motion, so the animation
// loop stops once the picture is still.
function applyGraphForces() {
  let energy = 0;
  for (const v of graphSim.vertices) {
    const p = graphSim.pos.get(v.id);
    if (!p || p.fixed) continue;
    const target = graphSim.pins.get(v.id) ?? graphSim.targets.get(v.id);
    if (!target) continue;
    const dx = (target.x - p.x) * GRAPH.ease;
    const dy = (target.y - p.y) * GRAPH.ease;
    p.x += dx;
    p.y += dy;
    energy += dx * dx + dy * dy;
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
    const placed = [];
    for (const { edge, visual, hit, label, cut } of graphSim.dom.edges) {
      setLine(visual, edge.a, edge.b);
      setLine(hit, edge.a, edge.b);
      const pa = P.get(edge.a);
      const pb = P.get(edge.b);
      if (pa && pb) {
        const angle = (Math.atan2(pb.y - pa.y, pb.x - pa.x) * 180) / Math.PI;
        cut.setAttribute("transform", `translate(${(pa.x + pb.x) / 2} ${(pa.y + pb.y) / 2}) rotate(${angle})`);
        // Sit the state label beside the line (not on it) so packets and the
        // dashed stroke do not cover it.
        const length = Math.hypot(pb.x - pa.x, pb.y - pa.y) || 1;
        const nx = -(pb.y - pa.y) / length;
        const ny = (pb.x - pa.x) / length;
        const side = ny > 0 ? -1 : 1;
        const width = Number(label.querySelector(".link-pill")?.getAttribute("width")) || 0;
        placed.push({ label, width, x: (pa.x + pb.x) / 2 + nx * 20 * side, y: (pa.y + pb.y) / 2 + ny * 20 * side });
      }
    }
    // Crossing lines can put two pills on top of each other: push the lower
    // one down until they no longer overlap.
    const visible = placed.filter((item) => item.width > 0).sort((a, b) => a.y - b.y);
    for (let i = 0; i < visible.length; i += 1) {
      for (let j = 0; j < i; j += 1) {
        const above = visible[j];
        const below = visible[i];
        const overlapX = Math.abs(above.x - below.x) < (above.width + below.width) / 2 + 4;
        if (overlapX && below.y - above.y < 24) below.y = above.y + 24;
      }
    }
    for (const item of placed) item.label.setAttribute("transform", `translate(${item.x} ${item.y})`);
    for (const { edge, line } of graphSim.dom.entityEdges) setLine(line, edge.a, edge.b);
    for (const { from, to, line } of graphSim.dom.flows) setLine(line, from, to);
  }
  const reduced = document.documentElement.dataset.reducedMotion === "true";
  const period = Math.max(300, Number(host.getStepMillis?.()) || 1150);
  const elapsed = now - (graphSim.packetEpoch ?? 0);
  const phase = (elapsed % period) / period;
  const cycle = Math.floor(elapsed / period);
  // "+N" badges sit a third of the way along their edge, clear of the label.
  for (const { a, b, badge } of graphSim.edgeBadges ?? []) {
    const pa = P.get(a);
    const pb = P.get(b);
    if (pa && pb) badge.setAttribute("transform", `translate(${pa.x + (pb.x - pa.x) * 0.33} ${pa.y + (pb.y - pa.y) * 0.33})`);
  }
  for (const packet of graphSim.packets) {
    const pa = P.get(packet.from);
    const pb = P.get(packet.to);
    if (!pa || !pb) continue;
    // Messages sharing an edge leave a little apart; every one arrives within
    // the step and then rests at its step's end point until the next cycle.
    const stagger = Math.min(packet.lane, 3) * 0.12;
    const t = reduced ? 0.5 : Math.min(Math.max((phase - stagger) / 0.6, 0), 1);
    const eased = t * t * (3 - 2 * t);
    let progress = packet.start + (packet.end - packet.start) * eased;
    let opacity = 1;
    if (packet.doomed) {
      // Dropped on delivery: it never reaches the target.
      progress = Math.min(progress, packet.start + (packet.end - packet.start) * 0.8, 0.82);
      opacity = reduced ? 0.55 : 1 - 0.85 * eased;
    }
    packet.dot.style.opacity = opacity;
    const x = pa.x + (pb.x - pa.x) * progress;
    const y = pa.y + (pb.y - pa.y) * progress;
    packet.dot.setAttribute("transform", `translate(${x} ${y})`);
    // Each cycle a delivered message ripples at its target, and a lost one
    // bursts where it vanishes. Control traffic stays silent. The arrival
    // point can fall in a short slice of the cycle that slow frames skip, so
    // an arrival missed in a finished cycle fires on the next frame.
    if (!reduced && packet.end >= 1 && packet.family !== "control") {
      const missed = packet.lastCycle !== undefined && cycle > packet.lastCycle && packet.firedCycle !== packet.lastCycle;
      if ((t >= 1 && packet.firedCycle !== cycle) || missed) {
        packet.firedCycle = t >= 1 ? cycle : packet.lastCycle;
        if (packet.doomed) {
          const stop = Math.min(packet.start + (packet.end - packet.start) * 0.8, 0.82);
          spawnLossBurst(pa.x + (pb.x - pa.x) * stop, pa.y + (pb.y - pa.y) * stop);
        } else {
          spawnArrivalRipple(packet.to, packet.family);
        }
      }
      packet.lastCycle = cycle;
    }
    // The key label reads mid-edge and fades near the nodes it would overlap.
    if (packet.label) packet.label.style.opacity = String(Math.min(1, Math.sin(Math.PI * progress) * 1.6));
    if (packet.trail) {
      packet.trail.style.opacity = opacity;
      const tail = Math.max(packet.start, progress - 0.16);
      packet.trail.setAttribute("x1", pa.x + (pb.x - pa.x) * tail);
      packet.trail.setAttribute("y1", pa.y + (pb.y - pa.y) * tail);
      packet.trail.setAttribute("x2", x);
      packet.trail.setAttribute("y2", y);
    }
  }
  updatePulses(now);
  applyDueNodes(now);
  applyDueFlashes(now);
  updateRoleOverlays(now);
  host.onPaint?.();
}

/** Where a vertex is drawn, in client (viewport) pixels; null when unknown. */
export function vertexClientPoint(id) {
  const p = graphSim.pos.get(id);
  const ctm = host?.svg.getScreenCTM();
  if (!p || !ctm) {
    return null;
  }
  const gx = p.x * graphSim.view.scale + graphSim.view.tx;
  const gy = p.y * graphSim.view.scale + graphSim.view.ty;
  return {
    x: ctm.a * gx + ctm.c * gy + ctm.e,
    y: ctm.b * gx + ctm.d * gy + ctm.f,
    radius: GRAPH.nodeRadius * graphSim.view.scale * ctm.a,
  };
}

// Transient, choreographed "data flow" triggered by a client push: request travels
// ---------------------------------------------------------------------------
// Role transitions as events: the leader's star travels from the previous
// leader to the new one, a new term floats up from the node that started it,
// and a campaigning candidate counts the vote responses as they land.
// ---------------------------------------------------------------------------
function announceRoleChange(node, status, shown, snapshot) {
  const layer = graphSim.dom?.roleLayer;
  if (!layer) return;
  const motion = !reducedMotion();
  if (status === "leader" && shown?.status !== "leader") {
    const from = graphSim.lastLeaderId;
    if (motion && from && from !== node.id && graphSim.pos.has(from)) {
      const baton = svg("g", { class: "leader-baton" });
      baton.append(svg("circle", { r: 13 }), svg("text", { y: 5 }, ROLE_BADGES.leader));
      layer.append(baton);
      const group = graphSim.dom.vertices.get(node.id)?.group;
      group?.classList.add("baton-incoming");
      graphSim.overlays.push({ kind: "baton", el: baton, from, to: node.id, t0: performance.now(), duration: 750, group });
    } else if (motion) {
      const group = graphSim.dom.vertices.get(node.id)?.group;
      group?.classList.remove("badge-pop");
      void group?.getBoundingClientRect();
      group?.classList.add("badge-pop");
    }
    graphSim.lastLeaderId = node.id;
  }
  const termRose = shown && Number(node.term) > Number(shown.term);
  // Only the node that opened the term (the one sending vote requests, or the
  // leader) announces it; voters move to the term silently.
  const opened = status === "leader" || graphSim.packets.some((packet) => packet.kind === "vote_request" && packet.from === node.id);
  if (motion && termRose && opened) {
    const label = svg("text", { class: "term-float" }, `term ${node.term}`);
    layer.append(label);
    graphSim.overlays.push({ kind: "term", el: label, nodeId: node.id, t0: performance.now(), duration: 1100 });
  }
}

// The node sending vote requests this step is the candidate; its tally counts
// its own vote plus each vote response as it arrives, against the quorum.
function syncVoteTally(snapshot) {
  const layer = graphSim.dom?.roleLayer;
  if (!layer) return;
  const requests = graphSim.packets.filter((packet) => packet.kind === "vote_request");
  const candidates = new Set(requests.map((packet) => packet.from));
  for (const [id, tally] of graphSim.tallies) {
    if (!candidates.has(id)) {
      tally.el.remove();
      graphSim.tallies.delete(id);
    }
  }
  for (const [id, item] of graphSim.dom.vertices) {
    if (item.vertex.kind === "node") item.group.classList.toggle("campaigning", candidates.has(id));
  }
  const quorum = Math.floor((snapshot.nodes || []).length / 2) + 1;
  for (const id of candidates) {
    const times = graphSim.packets
      .filter((packet) => packet.kind === "vote_response" && packet.to === id && !packet.doomed && packet.end >= 1)
      .map((packet) => (reducedMotion() ? 0 : packetArrivalTime(packet)));
    let tally = graphSim.tallies.get(id);
    if (!tally) {
      tally = { el: svg("text", { class: "vote-tally" }) };
      layer.append(tally.el);
      graphSim.tallies.set(id, tally);
    }
    Object.assign(tally, { times, quorum });
    tally.el.dataset.node = id;
  }
}

function spawnArrivalRipple(nodeId, family) {
  const layer = graphSim.dom?.roleLayer;
  if (!layer || !graphSim.pos.has(nodeId)) return;
  const ring = svg("circle", { class: `arrival-ripple ${family}`, r: GRAPH.nodeRadius });
  layer.prepend(ring);
  graphSim.overlays.push({ kind: "ripple", el: ring, nodeId, t0: performance.now(), duration: 520 });
}

function spawnLossBurst(x, y) {
  const layer = graphSim.dom?.roleLayer;
  if (!layer) return;
  const burst = svg("g", { class: "loss-burst" });
  burst.append(svg("line", { x1: -6, y1: -6, x2: 6, y2: 6 }), svg("line", { x1: -6, y1: 6, x2: 6, y2: -6 }));
  layer.append(burst);
  graphSim.overlays.push({ kind: "burst", el: burst, x, y, t0: performance.now(), duration: 480 });
}

function updateRoleOverlays(now) {
  const P = graphSim.pos;
  const remaining = [];
  for (const overlay of graphSim.overlays) {
    const t = Math.min((now - overlay.t0) / overlay.duration, 1);
    const eased = t * t * (3 - 2 * t);
    if (overlay.kind === "baton") {
      const a = P.get(overlay.from);
      const b = P.get(overlay.to);
      if (a && b) {
        const r = GRAPH.nodeRadius * 0.72;
        // An arc, so the hand-over reads as a pass rather than a slide.
        const lift = Math.sin(Math.PI * eased) * 46;
        const x = a.x + r + (b.x - a.x) * eased;
        const y = a.y - r + (b.y - a.y) * eased - lift;
        overlay.el.setAttribute("transform", `translate(${x} ${y})`);
      }
    } else if (overlay.kind === "ripple") {
      const p = P.get(overlay.nodeId);
      if (p) {
        overlay.el.setAttribute("cx", p.x);
        overlay.el.setAttribute("cy", p.y);
        overlay.el.setAttribute("r", GRAPH.nodeRadius + 18 * eased);
        overlay.el.style.opacity = String(0.7 * (1 - t));
      }
    } else if (overlay.kind === "burst") {
      overlay.el.setAttribute("transform", `translate(${overlay.x} ${overlay.y}) scale(${0.6 + 0.9 * eased})`);
      overlay.el.style.opacity = String(1 - t);
    } else if (overlay.kind === "term") {
      const p = P.get(overlay.nodeId);
      if (p) {
        overlay.el.setAttribute("x", p.x);
        overlay.el.setAttribute("y", p.y - GRAPH.nodeRadius - 14 - 22 * eased);
        overlay.el.style.opacity = String(1 - t * t);
      }
    }
    if (t < 1) {
      remaining.push(overlay);
    } else {
      overlay.el.remove();
      overlay.group?.classList.remove("baton-incoming");
    }
  }
  graphSim.overlays = remaining;
  for (const [id, tally] of graphSim.tallies) {
    const p = P.get(id);
    if (!p) continue;
    const votes = 1 + tally.times.filter((at) => now >= at).length;
    const text = `${Math.min(votes, tally.quorum)}/${tally.quorum} votes`;
    if (tally.el.textContent !== text) tally.el.textContent = text;
    tally.el.classList.toggle("won", votes >= tally.quorum);
    tally.el.setAttribute("x", p.x);
    tally.el.setAttribute("y", p.y + GRAPH.nodeRadius + 24);
  }
}

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
      // A pending copy stops short and lingers faintly; a lost one fades out
      // before its target and leaves a loss marker.
      const linger = mover.hold ? 900 : 0;
      if (progress >= 1 + linger / mover.durMs) {
        mover.dot.setAttribute("opacity", "0");
        if (mover.lost && !mover.burst) {
          mover.burst = true;
          const pa = P.get(mover.fromId);
          const pb = P.get(mover.toId);
          if (pa && pb) spawnLossBurst(pa.x + (pb.x - pa.x) * mover.end, pa.y + (pb.y - pa.y) * mover.end);
        }
        continue;
      }
      alive = true;
      const pa = P.get(mover.fromId);
      const pb = P.get(mover.toId);
      if (!pa || !pb) {
        continue;
      }
      const along = Math.min(progress, 1) * mover.end;
      const fade = mover.lost ? 1 - 0.8 * Math.min(progress, 1) : mover.hold && progress >= 1 ? 0.45 : 1;
      mover.dot.setAttribute("opacity", String(fade));
      mover.dot.setAttribute("cx", pa.x + (pb.x - pa.x) * along);
      mover.dot.setAttribute("cy", pa.y + (pb.y - pa.y) * along);
    }
    if (alive) {
      remaining.push(pulse);
    } else {
      for (const mover of pulse.movers) mover.dot.remove();
    }
  }
  graphSim.pulses = remaining;
}

// What a push did, read from the engine (schema 9 applied_at_step): which
// node took it first, when each replica applied it, which are still waiting
// for it, which will never get it, and when each subscriber was told.
//   after:  snapshot right after the push call (the push step)
//   final:  snapshot after the cluster settled
export function pushPlan(after, final, clientId, namespace, key) {
  const fullKey = `${namespace}:${key}`;
  const replicasAt = (snapshot) =>
    new Map(((snapshot?.keys || []).find((view) => view.key === fullKey)?.replicas || []).map((replica) => [replica.node_id, replica]));
  const pushStep = after.step;
  // A routed write (schema 11) carries its own record of the path it took.
  const routed = (final.writes || []).findLast(
    (write) => write.key === fullKey && write.client === clientId && write.started_at_step === pushStep,
  );
  if (routed) return routedPushPlan(final, routed, clientId, namespace, fullKey, replicasAt(final));
  const atPush = replicasAt(after);
  const entry = [...atPush.values()].find((replica) => replica.applied_at_step === pushStep)?.node_id ?? null;
  if (!entry) return null;
  const version = atPush.get(entry).version;
  const finalReplicas = replicasAt(final);
  const cut = new Set((final.links || []).filter((link) => link.state === "partitioned").map((link) => `${link.from}>${link.to}`));
  const replicas = (final.nodes || [])
    .filter((node) => node.id !== entry)
    .map((node) => {
      const replica = finalReplicas.get(node.id);
      if (replica && replica.version === version && Number.isFinite(replica.applied_at_step) && replica.applied_at_step > pushStep) {
        return { node: node.id, status: "applied", steps: replica.applied_at_step - pushStep };
      }
      // Still on its way only if a write is in flight to it and will arrive.
      const waiting =
        !node.crashed &&
        !node.disabled &&
        (final.in_flight || []).some(
          (message) =>
            message.to === node.id &&
            message.key === fullKey &&
            String(message.kind).startsWith("replicate_") &&
            !cut.has(`${message.from}>${message.to}`),
        );
      return { node: node.id, status: waiting ? "pending" : "lost", steps: null };
    });
  const subscribers = (final.subscribers || [])
    .filter((subscriber) => subscriber.namespace === namespace && subscriber.last_event?.key === fullKey)
    .map((subscriber) => ({
      id: subscriber.id,
      node: subscriber.connected_node,
      steps: Math.max(0, subscriber.last_event.delivered_at_step - pushStep),
    }));
  // Since schema 10 the entry node is the member the writer is connected to
  // (its coordinator): the request goes there directly, never via the leader.
  return { clientId, entry, pushStep, replicas, subscribers };
}

// The plan for a routed write: client -> coordinator, a forward to the primary
// when the coordinator is not it, the primary's copies to its backups, and in
// local_first the invalidations to every other member. Each hop is timed by
// the step the engine applied it (applied_at_step) or shown waiting or lost.
function routedPushPlan(final, write, clientId, namespace, fullKey, finalReplicas) {
  const pushStep = write.started_at_step;
  const nodesById = new Map((final.nodes || []).map((node) => [node.id, node]));
  const cut = new Set((final.links || []).filter((link) => link.state === "partitioned").map((link) => `${link.from}>${link.to}`));
  const appliedAfter = (node) => {
    const replica = finalReplicas.get(node);
    return replica && Number.isFinite(replica.applied_at_step) && replica.applied_at_step >= pushStep && replica.version
      ? replica.applied_at_step - pushStep
      : null;
  };
  const hop = (from, node, after) => {
    const steps = appliedAfter(node);
    if (steps !== null) return { from, node, status: "applied", steps: Math.max(1, steps - after), after };
    const target = nodesById.get(node);
    const waiting =
      target &&
      !target.crashed &&
      !target.disabled &&
      (final.in_flight || []).some(
        (message) => message.to === node && message.key === fullKey && String(message.kind).startsWith("replicate_") && !cut.has(`${message.from}>${message.to}`),
      );
    return { from, node, status: waiting ? "pending" : "lost", steps: null, after };
  };
  const entry = write.coordinator;
  const replicas = [];
  let primaryAt = 0;
  if (write.primary !== entry) {
    const forward = { ...hop(entry, write.primary, 0), kind: "data forward" };
    replicas.push(forward);
    primaryAt = forward.status === "applied" ? forward.steps : 0;
  }
  for (const backup of write.backups) replicas.push(hop(write.primary, backup, primaryAt));
  if (final.replication?.model === "local_first") {
    for (const node of final.nodes || []) {
      if (node.id === entry || node.disabled) continue;
      const down = node.crashed || cut.has(`${entry}>${node.id}`);
      replicas.push({ from: entry, node: node.id, status: down ? "lost" : "applied", steps: 1, after: 0, kind: "event" });
    }
  }
  const subscribers = (final.subscribers || [])
    .filter((subscriber) => subscriber.namespace === namespace && subscriber.last_event?.key === fullKey)
    .map((subscriber) => ({
      id: subscriber.id,
      node: subscriber.connected_node,
      steps: Math.max(0, subscriber.last_event.delivered_at_step - pushStep),
    }));
  return { clientId, entry, pushStep, replicas, subscribers, write };
}

const PULSE_STEP_MS = 420;
const PULSE_REQUEST_MS = 460;

// Play a push as the engine ran it (pushPlan): the request reaches the entry
// node, each replica's copy travels as many steps as the engine took to
// deliver it, a copy still on its way stops short, a lost one fades with a
// loss marker, and subscribers hear about it when their event was delivered.
// Vertices flash when their message arrives (flashWhenReached).
export function playDataFlowPulse(plan) {
  if (!graphSim.dom || !plan) {
    return;
  }
  if (document.documentElement.dataset.reducedMotion === "true") {
    return;
  }
  const movers = [];
  const add = (fromId, toId, startMs, durMs, kind, extra = {}) => {
    if (!graphSim.pos.has(fromId) || !graphSim.pos.has(toId)) {
      return;
    }
    const dot = svg("circle", { class: `pulse ${kind}`, r: 6, opacity: 0, cx: 0, cy: 0 });
    dot.dataset.from = fromId;
    dot.dataset.to = toId;
    graphSim.dom.pulseLayer.append(dot);
    movers.push({ fromId, toId, startMs, durMs, kind, dot, end: 1, ...extra });
  };

  const clientVertex = `client:${plan.clientId}`;
  // 1) the request into the cluster, along the writer's link to the member
  //    it is connected to, which is the engine's entry node for the write.
  //    A brand-new writer has no vertex yet, so it starts at its node.
  const requestMs = graphSim.pos.has(clientVertex) ? PULSE_REQUEST_MS : 0;
  if (requestMs) add(clientVertex, plan.entry, 0, requestMs, "data");
  // 2) replication, timed by the steps the engine took
  const replicationStart = Math.max(0, requestMs - 100);
  for (const replica of plan.replicas) {
    // Routed writes hop from the primary, and start once it has the value.
    const from = replica.from ?? plan.entry;
    const start = replicationStart + PULSE_STEP_MS * Math.min(replica.after ?? 0, 6);
    const kind = replica.kind ?? "data";
    if (replica.status === "applied") {
      const steps = Math.min(replica.steps, 6);
      add(from, replica.node, start, PULSE_STEP_MS * steps, kind);
    } else if (replica.status === "pending") {
      add(from, replica.node, start, PULSE_STEP_MS * 2, `${kind} pending`, { end: 0.55, hold: true });
    } else {
      add(from, replica.node, start, PULSE_STEP_MS, `${kind} doomed`, { end: 0.8, lost: true });
    }
  }
  // 3) delivery to each subscriber, from its node, when the engine delivered
  for (const subscriber of plan.subscribers) {
    const start = replicationStart + PULSE_STEP_MS * Math.max(1, Math.min(subscriber.steps, 6));
    add(subscriber.node, `sub:${subscriber.id}`, start, 520, "event");
  }

  if (movers.length === 0) {
    return;
  }
  const t0 = performance.now();
  graphSim.pulses.push({ t0, movers });
  if (requestMs) {
    graphSim.pulseArrivals.set(plan.entry, t0 + requestMs);
  } else {
    graphSim.pulseArrivals.delete(plan.entry);
  }
  // Each vertex lights up when its message arrives; pending and lost
  // copies never arrive, so their nodes do not flash.
  for (const mover of movers) {
    if (mover.end < 1 || mover.fromId === clientVertex) continue;
    const at = t0 + mover.startMs + mover.durMs;
    graphSim.pulseArrivals.set(mover.toId, Math.max(graphSim.pulseArrivals.get(mover.toId) ?? 0, at));
    if (mover.toId.startsWith("sub:")) {
      const group = graphSim.dom.vertices.get(mover.toId)?.group;
      if (group) graphSim.pendingFlashes.set(mover.toId, { at, group, name: "received" });
    }
  }
  // A node whose copy is pending or lost must not flash on this push.
  for (const mover of movers) {
    if (mover.end < 1 && !movers.some((other) => other.toId === mover.toId && other.end === 1)) {
      graphSim.pulseArrivals.set(mover.toId, Number.POSITIVE_INFINITY);
    }
  }
  startGraphSim();
}

function startGraphSim() {
  if (graphSim.raf !== null) return;
  const tick = (now) => {
    const energy = applyGraphForces();
    const fitError = easeTowardFit();
    paintGraph(now);
    const busy =
      graphSim.drag ||
      graphSim.packets.length > 0 ||
      graphSim.pulses.length > 0 ||
      graphSim.pendingNodes.size > 0 ||
      graphSim.pendingFlashes.size > 0 ||
      graphSim.overlays.length > 0 ||
      energy > 0.05 ||
      fitError > 0.5;
    graphSim.raf = busy ? requestAnimationFrame(tick) : null;
    if (!busy) paintGraph(now); // final settle paint
  };
  graphSim.raf = requestAnimationFrame(tick);
}

function pointerToGraph(event) {
  const ctm = host.svg.getScreenCTM();
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
    holdTextSelection(true);
    startGraphSim();
    capturePointer(group, event.pointerId);

    const move = (ev) => {
      if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) > 4) graphSim.movedDuringPress = true;
      const g = pointerToGraph(ev);
      p.x = g.x;
      p.y = g.y;
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
      holdTextSelection(false);
      // A vertex the user moved stays where it was put (until "Tidy").
      if (graphSim.movedDuringPress) graphSim.pins.set(vertex.id, { x: p.x, y: p.y });
      if (group.hasPointerCapture?.(ev.pointerId)) group.releasePointerCapture(ev.pointerId);
      group.removeEventListener("pointermove", move);
      group.removeEventListener("pointerup", up);
      group.removeEventListener("pointercancel", cancel);
      group.removeEventListener("lostpointercapture", cancel);
      if (completed && !graphSim.movedDuringPress && vertex.kind === "subscriber") {
        host.onSubscriberPress?.(vertex.ref);
      }
      if (completed && !graphSim.movedDuringPress && vertex.kind === "client") {
        host.onClientPress(vertex.ref);
      }
      if (completed && !graphSim.movedDuringPress && vertex.kind === "node") {
        host.onNodePress?.(vertex.id);
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

// A drag on the graph must not select text (node labels, or page text the
// pointer passes over): drop any selection it started and suppress selection
// on the whole page until the drag ends (CSS: html.graph-dragging).
function holdTextSelection(on) {
  document.documentElement.classList.toggle("graph-dragging", on);
  if (on) window.getSelection?.()?.removeAllRanges();
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
  host.svg.addEventListener("pointerdown", (event) => {
    if (event.target.closest(".vertex") || event.target.closest(".link-hit")) return;
    const ctm = host.svg.getScreenCTM();
    const sx = ctm && ctm.a ? ctm.a : 1;
    const sy = ctm && ctm.d ? ctm.d : 1;
    const start = { x: event.clientX, y: event.clientY, tx: graphSim.view.tx, ty: graphSim.view.ty };
    graphSim.drag = { pan: true };
    holdTextSelection(true);
    capturePointer(host.svg, event.pointerId);
    // A plain click (e.g. to dismiss a popover) is not a pan: the view stays
    // fitted until the pointer really moves, so the cluster keeps re-centring
    // as it grows or settles.
    let panning = false;
    const move = (ev) => {
      if (!panning) {
        if (Math.hypot(ev.clientX - start.x, ev.clientY - start.y) <= 4) return;
        panning = true;
        setAutoFit(false);
        // Pan from wherever the fitted view eased to, not from the press.
        start.tx = graphSim.view.tx;
        start.ty = graphSim.view.ty;
        start.x = ev.clientX;
        start.y = ev.clientY;
      }
      graphSim.view.tx = start.tx + (ev.clientX - start.x) / sx;
      graphSim.view.ty = start.ty + (ev.clientY - start.y) / sy;
      paintGraph(performance.now());
    };
    let finished = false;
    const up = (ev) => {
      if (finished) return;
      finished = true;
      graphSim.drag = null;
      holdTextSelection(false);
      if (host.svg.hasPointerCapture?.(ev.pointerId)) host.svg.releasePointerCapture(ev.pointerId);
      host.svg.removeEventListener("pointermove", move);
      host.svg.removeEventListener("pointerup", up);
      host.svg.removeEventListener("pointercancel", up);
      host.svg.removeEventListener("lostpointercapture", up);
    };
    host.svg.addEventListener("pointermove", move);
    host.svg.addEventListener("pointerup", up);
    host.svg.addEventListener("pointercancel", up);
    host.svg.addEventListener("lostpointercapture", up);
  });
  // Ctrl/Cmd + wheel (and trackpad pinch, which browsers report the same way)
  // zooms around the cursor. A plain wheel scrolls the page as usual.
  host.svg.addEventListener(
    "wheel",
    (event) => {
      if (!event.ctrlKey && !event.metaKey) {
        return;
      }
      event.preventDefault();
      setAutoFit(false);
      const ctm = host.svg.getScreenCTM();
      if (!ctm) return;
      const inv = ctm.inverse();
      const anchor = {
        x: inv.a * event.clientX + inv.c * event.clientY + inv.e,
        y: inv.b * event.clientX + inv.d * event.clientY + inv.f,
      };
      zoomAround(anchor, event.deltaY < 0 ? 1.1 : 1 / 1.1);
    },
    { passive: false },
  );
}

// Glyph drawn in the corner badge of a node, so the role reads without colour.
const ROLE_BADGES = {
  leader: "★",
  candidate: "?",
  crashed: "✕",
  joining: "+",
  catching_up: "↻",
};

// Drawn badge icons (stroked paths in a 26px badge): a power symbol for a
// disabled node, and a slashed circle for one cut off from the cluster (the
// engine reports an isolated node as "disconnected").
const BADGE_ICONS = {
  disabled: "M-4.4 -3.6 A6 6 0 1 0 4.4 -3.6 M0 -7 V-0.5",
  disconnected: "M0 -6.5 A6.5 6.5 0 1 1 -0.01 -6.5 Z M-4.6 4.6 L4.6 -4.6",
};

function setHitTitle(hit, text) {
  let title = hit.querySelector("title");
  if (!title) {
    title = svg("title");
    hit.append(title);
  }
  if (title.textContent !== text) title.textContent = text;
}

// One dashed line per writer and subscriber that share a namespace.
function syncNamespaceFlows(snapshot) {
  const dom = graphSim.dom;
  const pairs = [];
  for (const client of snapshot.clients || []) {
    for (const subscriber of snapshot.subscribers || []) {
      if (client.namespace === subscriber.namespace) {
        pairs.push({ from: `client:${client.id}`, to: `sub:${subscriber.id}`, ns: client.namespace });
      }
    }
  }
  const key = pairs.map((pair) => `${pair.from}>${pair.to}`).join(",");
  if (key === dom.flowKey) return;
  dom.flowKey = key;
  dom.flows = pairs
    .filter((pair) => graphSim.pos.has(pair.from) && graphSim.pos.has(pair.to))
    .map((pair) => {
      const line = svg("line", { class: "namespace-flow" });
      line.dataset.namespace = pair.ns;
      return { ...pair, line };
    });
  dom.flowLayer.replaceChildren(...dom.flows.map((flow) => flow.line));
}

// Messages a node pair dropped (both directions), by cause, from the
// snapshot's per-link counters (schema 9); zeros on older snapshots.
export function pairDrops(link, reverse) {
  const read = (view, field) => Number(view?.drops?.[field]) || 0;
  const sum = (field) => read(link, field) + read(reverse, field);
  const refused = sum("cut_at_send") + sum("cut_at_delivery");
  const injected = sum("injected");
  // Delivered to a down node, per end: a burst shows at that node.
  const downAt = { [link.to]: read(link, "target_down"), [link.from]: read(reverse, "target_down") };
  return { refused, injected, downAt, total: refused + injected + downAt[link.to] + downAt[link.from] };
}

// "partitioned", or "delayed · N ms" with the longest pending delay on the
// pair (it counts down as the delayed messages approach delivery).
function linkLabelText(edge) {
  if (edge.state === "up") return "";
  if (edge.state === "delayed" && edge.delay > 0) return `delayed · ${edge.delay} ms`;
  // A cut link says how much traffic it has refused (cumulative for the run).
  if (edge.state === "partitioned" && edge.drops?.refused > 0) return `partitioned · ${edge.drops.refused} refused`;
  return edge.state;
}

function dropsText(drops) {
  if (!drops || drops.total === 0) return "no messages dropped";
  const parts = [];
  if (drops.refused) parts.push(`${drops.refused} refused by the cut`);
  if (drops.injected) parts.push(`${drops.injected} dropped by an injected fault`);
  for (const [node, count] of Object.entries(drops.downAt)) {
    if (count) parts.push(`${count} lost at ${node} while it was down`);
  }
  return parts.join(", ");
}

// When a pair's drop counters rise, burst a loss marker where it happened:
// at the cut for refused or injected drops, at the node for messages that
// reached it while it was down.
function burstNewDrops(edge) {
  const previous = graphSim.dropTotals.get(`${edge.a}|${edge.b}`);
  graphSim.dropTotals.set(`${edge.a}|${edge.b}`, edge.drops);
  if (!previous || reducedMotion()) return;
  const pa = graphSim.pos.get(edge.a);
  const pb = graphSim.pos.get(edge.b);
  if (!pa || !pb) return;
  if (edge.drops.refused + edge.drops.injected > previous.refused + previous.injected) {
    spawnLossBurst((pa.x + pb.x) / 2, (pa.y + pb.y) / 2);
  }
  for (const [node, count] of Object.entries(edge.drops.downAt)) {
    if (count > (previous.downAt[node] ?? 0)) {
      const [from, to] = node === edge.b ? [pa, pb] : [pb, pa];
      spawnLossBurst(from.x + (to.x - from.x) * 0.78, from.y + (to.y - from.y) * 0.78);
    }
  }
}

function setLinkLabel(label, text) {
  const textNode = label.querySelector(".link-text");
  if (textNode.textContent === text) return;
  textNode.textContent = text;
  const pill = label.querySelector(".link-pill");
  if (!text) {
    pill.setAttribute("width", 0);
    return;
  }
  // Size the pill from the rendered text; fall back to an estimate when the
  // label is not laid out yet (hidden canvas).
  let width = 0;
  try {
    width = textNode.getComputedTextLength();
  } catch {
    width = 0;
  }
  width = (width || text.length * 7.4) + 16;
  pill.setAttribute("x", (-width / 2).toFixed(1));
  pill.setAttribute("y", -10);
  pill.setAttribute("width", width.toFixed(1));
  pill.setAttribute("height", 20);
}

// When collapsing the two directed links of a pair into one drawn line, show the
// most degraded state so a partition/delay is never hidden by a healthy reverse.
function worstLinkState(a, b) {
  const rank = { partitioned: 3, delayed: 2, up: 1 };
  const left = rank[a] || 0;
  const right = rank[b] || 0;
  return right > left ? b : a;
}

// Message family drives colour and shape: votes, data (writes being
// replicated), events (invalidations and flushes), and control traffic
// (heartbeats and acks).
export function packetFamily(kind) {
  const normalized = String(kind || "").toLowerCase();
  if (normalized.includes("vote")) return "vote";
  if (normalized.includes("heartbeat") || normalized === "ack") return "control";
  if (normalized.includes("put") || normalized.includes("upsert") || normalized.includes("data")) return "data";
  if (normalized.includes("invalid") || normalized.includes("flush") || normalized.includes("event")) return "event";
  return "message";
}

function packetKind(kind) {
  const normalized = String(kind || "message")
    .trim()
    .toLowerCase()
    .replace(/_/g, "-")
    .replace(/[^a-z0-9-]/g, "-");
  const family = packetFamily(kind);
  // "heartbeat" keeps its colour class; "control" marks it quiet.
  if (family === "control") return `control heartbeat ${normalized}`;
  return family === "message" ? normalized || "message" : `${family} ${normalized}`;
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
