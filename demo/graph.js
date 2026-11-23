// Force-directed cluster graph (presentation only: positions carry no engine
// meaning). The page passes in its SVG element and a few hooks; the graph
// never reads page state directly.
import { nodeStatus } from "./insight.js";

const SVG_NS = "http://www.w3.org/2000/svg";

// host: { svg, getSnapshot(), getSelectedLink(), onSelectLink(link), onClientPress(client) }
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

export function resetGraph() {
  seenNodes = new Set();
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
  host.svg.replaceChildren();
  const viewport = svg("g", { class: "graph-viewport" });
  const edgeLayer = svg("g", { class: "links" });
  const entityLinkLayer = svg("g", { class: "entity-links", "aria-hidden": "true" });
  const packetLayer = svg("g", { class: "packet-layer", "aria-hidden": "true" });
  const pulseLayer = svg("g", { class: "pulse-layer", "aria-hidden": "true" });
  const vertexLayer = svg("g", { class: "vertices" });
  viewport.append(edgeLayer, entityLinkLayer, packetLayer, pulseLayer, vertexLayer);
  host.svg.append(viewport);

  const dom = { viewport, packetLayer, pulseLayer, edges: [], entityEdges: [], vertices: new Map() };

  for (const edge of graphSim.edges) {
    if (edge.kind === "node") {
      const visual = svg("line", { class: "link" });
      // The wide invisible hit line is also the keyboard target for the link.
      const hit = svg("line", { class: "link-hit", tabindex: 0, role: "button" });
      hit.addEventListener("click", (event) => {
        event.stopPropagation();
        host.onSelectLink({ from: edge.from, to: edge.to });
      });
      onActivateKey(hit, () => host.onSelectLink({ from: edge.from, to: edge.to }));
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
      if (vertex.kind === "client") {
        group.append(svg("title", {}, "Click or press Enter to push (drag to move)"));
        group.setAttribute("tabindex", "0");
        group.setAttribute("role", "button");
        onActivateKey(group, () => host.onClientPress(vertex.ref));
      } else {
        group.setAttribute("role", "img");
      }
    }
    if (vertex.kind === "node") {
      group.setAttribute("role", "img");
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
      const isNew = !seenNodes.has(node.id);
      seenNodes.add(node.id);
      item.group.setAttribute(
        "class",
        ["vertex", "node", `role-${status}`, node.crashed ? "crashed" : "", node.disabled ? "disabled" : "", isNew ? "joining" : ""].filter(Boolean).join(" "),
      );
      item.group.querySelector(".v-id").textContent = node.id;
      item.group.querySelector(".v-sub").textContent = status;
      item.group.setAttribute("aria-label", `${node.id}, ${status}, term ${node.term}`);
    } else {
      // Vertices are rebuilt only on topology change, so read the live entity.
      const ref =
        (v.kind === "client" ? snapshot.clients : snapshot.subscribers)?.find((entity) => entity.id === v.ref.id) ??
        v.ref;
      v.ref = ref;
      const active = v.kind === "client" ? Boolean(ref.last_op) : Boolean(ref.last_event);
      item.group.setAttribute("class", ["vertex", "entity", v.kind, active ? "active" : "", v.kind === "client" ? "clickable" : ""].filter(Boolean).join(" "));
      item.group.querySelector(".entity-id").textContent = v.kind === "client" ? ref.id : ref.client_id || ref.id;
      item.group.querySelector(".entity-sub").textContent = ref.namespace || "";
      item.group.setAttribute(
        "aria-label",
        v.kind === "client"
          ? `client ${ref.id} on ${ref.namespace}; press Enter to push`
          : `subscriber ${ref.client_id || ref.id} on ${ref.namespace}`,
      );
    }
  }
  // The DOM keeps the edge objects from when it was built (topology change
  // only); take state from the current edges so partitions and delays show.
  const liveEdges = new Map(graphSim.edges.map((edge) => [`${edge.a}>${edge.b}`, edge]));
  const current = host.getSelectedLink();
  for (const { edge: built, visual, hit } of graphSim.dom.edges) {
    const edge = liveEdges.get(`${built.a}>${built.b}`) ?? built;
    const selected = current && current.from === edge.from && current.to === edge.to;
    visual.setAttribute("class", ["link", edge.state, selected ? "selected" : ""].filter(Boolean).join(" "));
    hit.setAttribute("aria-label", `link ${edge.from} to ${edge.to}, ${edge.state}${selected ? ", selected" : ""}`);
    hit.setAttribute("aria-pressed", selected ? "true" : "false");
  }
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

export function playDataFlowPulse(clientId, namespace) {
  if (!graphSim.dom || !host.getSnapshot()) {
    return;
  }
  if (document.documentElement.dataset.reducedMotion === "true") {
    return;
  }
  const snapshot = host.getSnapshot();
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
        host.onClientPress(vertex.ref);
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
  host.svg.addEventListener("pointerdown", (event) => {
    if (event.target.closest(".vertex") || event.target.closest(".link-hit")) return;
    const ctm = host.svg.getScreenCTM();
    const sx = ctm && ctm.a ? ctm.a : 1;
    const sy = ctm && ctm.d ? ctm.d : 1;
    const start = { x: event.clientX, y: event.clientY, tx: graphSim.view.tx, ty: graphSim.view.ty };
    graphSim.drag = { pan: true };
    capturePointer(host.svg, event.pointerId);
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
  // Wheel zoom around the cursor.
  host.svg.addEventListener(
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

// When collapsing the two directed links of a pair into one drawn line, show the
// most degraded state so a partition/delay is never hidden by a healthy reverse.
function worstLinkState(a, b) {
  const rank = { partitioned: 3, delayed: 2, up: 1 };
  const left = rank[a] || 0;
  const right = rank[b] || 0;
  return right > left ? b : a;
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
