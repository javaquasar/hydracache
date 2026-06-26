const DEFAULT_SEED = 80;
const DEFAULT_SCENARIO = "default";
const DEFAULT_ENGINE = "wasm";
const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const FNV_MASK = 0xffffffffffffffffn;

export function readInitialState(search) {
  const params = new URLSearchParams(search);
  const script = readReplayScript(params.get("script"));
  return {
    seed: script?.seed ?? readPositiveInt(params.get("seed"), DEFAULT_SEED),
    steps: readPositiveInt(params.get("steps"), 0),
    scenario: script?.scenario ?? readScenario(params.get("scenario")),
    mode: script?.mode ?? "manual",
    engine: readEngine(params.get("engine")),
    apiBase: readApiBase(params.get("api")),
    script,
  };
}

export function writeUrlState(
  history,
  snapshot,
  scenario = DEFAULT_SCENARIO,
  engine = DEFAULT_ENGINE,
  apiBase = "",
  replayScriptJson = "",
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
  const replayScript = readReplayScriptJson(replayScriptJson);
  if (shouldShareReplayScript(replayScript)) {
    params.set("script", encodeReplayScript(replayScript));
  }
  history.replaceState(null, "", `?${params.toString()}`);
}

export function reproducerCommand(seed, steps, replayScriptJson = "", currentUrl = "") {
  const replayScript = readReplayScriptJson(replayScriptJson);
  if (shouldShareReplayScript(replayScript) && currentUrl) {
    const url = new URL(currentUrl);
    url.search = "";
    url.searchParams.set("script", encodeReplayScript(replayScript));
    return url.toString();
  }
  return `cargo run -p hydracache-sim --bin vopr -- --seed ${seed} --steps ${steps}`;
}

export function encodeReplayScript(script) {
  return base64UrlEncode(JSON.stringify(script));
}

export function decodeReplayScript(encoded) {
  return readReplayScript(encoded);
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

function shouldShareReplayScript(script) {
  if (!script) {
    return false;
  }
  return (
    script.mode !== "manual" ||
    Boolean(script.scenario) ||
    (Array.isArray(script.actions) && script.actions.length > 0)
  );
}

function readReplayScriptJson(value) {
  if (!value) {
    return null;
  }
  try {
    return normalizeReplayScript(JSON.parse(value));
  } catch (_error) {
    return null;
  }
}

function readReplayScript(value) {
  if (!value) {
    return null;
  }
  try {
    return normalizeReplayScript(JSON.parse(base64UrlDecode(value)));
  } catch (_error) {
    return null;
  }
}

function normalizeReplayScript(value) {
  if (!value || typeof value !== "object") {
    return null;
  }
  if (value.version !== 1) {
    return null;
  }
  const seed = Number(value.seed);
  if (!Number.isSafeInteger(seed) || seed < 0) {
    return null;
  }
  const mode = ["manual", "scripted", "mixed"].includes(value.mode) ? value.mode : "manual";
  const scenario =
    value.scenario === null || value.scenario === undefined ? null : readScenario(value.scenario);
  const actions = Array.isArray(value.actions) ? value.actions : [];
  if (actions.length > 256) {
    return null;
  }
  return {
    version: 1,
    seed,
    mode,
    scenario,
    actions,
  };
}

function base64UrlEncode(value) {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
}

function base64UrlDecode(value) {
  const padded = value.replace(/-/g, "+").replace(/_/g, "/").padEnd(
    Math.ceil(value.length / 4) * 4,
    "=",
  );
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
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
