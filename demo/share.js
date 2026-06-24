const DEFAULT_SEED = 80;
const DEFAULT_SCENARIO = "default";
const DEFAULT_ENGINE = "wasm";
const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const FNV_MASK = 0xffffffffffffffffn;

export function readInitialState(search) {
  const params = new URLSearchParams(search);
  return {
    seed: readPositiveInt(params.get("seed"), DEFAULT_SEED),
    steps: readPositiveInt(params.get("steps"), 0),
    scenario: readScenario(params.get("scenario")),
    engine: readEngine(params.get("engine")),
    apiBase: readApiBase(params.get("api")),
  };
}

export function writeUrlState(
  history,
  snapshot,
  scenario = DEFAULT_SCENARIO,
  engine = DEFAULT_ENGINE,
  apiBase = "",
) {
  if (!history || !snapshot) {
    return;
  }
  const params = new URLSearchParams();
  params.set("seed", String(snapshot.seed));
  params.set("steps", String(snapshot.step));
  params.set("scenario", readScenario(scenario));
  if (readEngine(engine) === "server") {
    params.set("engine", "server");
    const normalizedApiBase = readApiBase(apiBase);
    if (normalizedApiBase) {
      params.set("api", normalizedApiBase);
    }
  }
  history.replaceState(null, "", `?${params.toString()}`);
}

export function reproducerCommand(seed, steps) {
  return `cargo run -p hydracache-sim --bin vopr -- --seed ${seed} --steps ${steps}`;
}

export function snapshotHash(snapshot) {
  const stable = stableJson(snapshot);
  let hash = FNV_OFFSET;
  for (let index = 0; index < stable.length; index += 1) {
    hash ^= BigInt(stable.charCodeAt(index));
    hash = (hash * FNV_PRIME) & FNV_MASK;
  }
  return hash.toString(16).padStart(16, "0");
}

function readPositiveInt(value, fallback) {
  const parsed = Number.parseInt(value ?? "", 10);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : fallback;
}

function readScenario(value) {
  const scenario = String(value ?? DEFAULT_SCENARIO).trim();
  return /^[a-z0-9_-]+$/i.test(scenario) ? scenario : DEFAULT_SCENARIO;
}

function readEngine(value) {
  return String(value ?? DEFAULT_ENGINE).trim().toLowerCase() === "server"
    ? "server"
    : DEFAULT_ENGINE;
}

function readApiBase(value) {
  const apiBase = String(value ?? "").trim().replace(/\/+$/, "");
  if (!apiBase) {
    return "";
  }
  return /^https?:\/\/[^\s]+$/i.test(apiBase) || apiBase.startsWith("/") ? apiBase : "";
}

function stableJson(value) {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  return `{${Object.keys(value)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`)
    .join(",")}}`;
}
