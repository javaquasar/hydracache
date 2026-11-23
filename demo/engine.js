// Engine sessions with one interface: the in-browser WASM simulator and the
// sandbox HTTP simulator (`/sim/*`). Every call resolves to engine state or
// throws; nothing here renders or guesses.

export class WasmSimSession {
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

export class ServerSimSession {
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

export function toBigInt(value) {
  return typeof value === "bigint" ? value : BigInt(value);
}

export function toNumber(value) {
  const number = Number(value);
  return Number.isSafeInteger(number) && number >= 0 ? number : 0;
}
