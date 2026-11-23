const DEFAULT_SEED = 80;
const DEFAULT_SCENARIO = "default";
const DEFAULT_ENGINE = "wasm";
const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const FNV_MASK = 0xffffffffffffffffn;

export const MAX_SEED = Number.MAX_SAFE_INTEGER;
const LOOPBACK_HOSTS = new Set(["localhost", "127.0.0.1", "[::1]"]);

// `warnings` lists URL parameters that were present but unusable, so the page
// can say so instead of silently running something else. `apiError` is set when
// a server engine URL is refused; the page must not fall back to another engine.
export function readInitialState(search, pageOrigin = "") {
  const params = new URLSearchParams(search);
  const warnings = [];
  const script = readReplayScript(params.get("script"));
  if (params.has("script") && !script) {
    warnings.push("URL replay script is invalid or uses an unsupported version; it was ignored.");
  }
  const steps = readPositiveInt(params.get("steps"), 0);
  if (params.has("steps") && !isWholeNumber(params.get("steps"))) {
    warnings.push(`URL steps "${params.get("steps")}" is not a whole number; using 0.`);
  }
  if (!script && params.has("seed") && !isWholeNumber(params.get("seed"))) {
    warnings.push(
      `URL seed "${params.get("seed")}" is not a whole number from 0 to ${MAX_SEED}; using ${DEFAULT_SEED}.`,
    );
  }
  const api = validateApiBase(params.get("api"), pageOrigin);
  // Links shared before replay scripts carried a target step kept it in `steps`.
  if (script && script.target_step === null && params.has("steps")) {
    script.target_step = steps;
  }
  return {
    seed: script?.seed ?? readPositiveInt(params.get("seed"), DEFAULT_SEED),
    steps,
    scenario: script?.scenario ?? readScenario(params.get("scenario")),
    mode: script?.mode ?? "manual",
    engine: readEngine(params.get("engine")),
    apiBase: api.apiBase,
    apiError: api.error,
    script,
    warnings,
  };
}

// Parse a seed typed by the user. Returns null for anything that is not a whole
// number the simulator can round-trip exactly through JSON snapshots.
export function parseSeed(value) {
  const text = String(value ?? "").trim();
  return isWholeNumber(text) ? Number.parseInt(text, 10) : null;
}

// Only same-origin paths, the page's own origin, or loopback simulator APIs are
// accepted: a shared link must not point the lab at someone else's server, which
// could render any verdict it likes.
export function validateApiBase(value, pageOrigin = "") {
  const apiBase = normalizeApiBase(value);
  if (!apiBase) {
    return { apiBase: "", error: "" };
  }
  if (apiBase.startsWith("/") && !apiBase.startsWith("//")) {
    return { apiBase, error: "" };
  }
  let url;
  try {
    url = new URL(apiBase);
  } catch (_error) {
    return { apiBase: "", error: `simulator API "${apiBase}" is not a valid URL` };
  }
  const allowed =
    (url.protocol === "http:" || url.protocol === "https:") &&
    (LOOPBACK_HOSTS.has(url.hostname) || (pageOrigin && url.origin === pageOrigin));
  return allowed
    ? { apiBase, error: "" }
    : {
        apiBase: "",
        error: `simulator API "${apiBase}" was refused: only same-origin or loopback (localhost, 127.0.0.1, [::1]) APIs are allowed`,
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
    const normalizedApiBase = normalizeApiBase(apiBase);
    if (normalizedApiBase) {
      params.set("api", normalizedApiBase);
    }
  }
  const replayScript = readReplayScriptJson(replayScriptJson);
  if (shouldShareReplayScript(replayScript)) {
    replayScript.target_step = snapshot.step;
    params.set("script", encodeReplayScript(replayScript));
  }
  history.replaceState(null, "", `?${params.toString()}`);
}

export function reproducerCommand(seed, steps, replayScriptJson = "", currentUrl = "") {
  const replayScript = readReplayScriptJson(replayScriptJson);
  if (shouldShareReplayScript(replayScript) && currentUrl) {
    replayScript.target_step = readPositiveInt(String(steps), replayScript.target_step ?? 0);
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
  const targetStep = value.target_step === null || value.target_step === undefined
    ? null
    : Number(value.target_step);
  if (targetStep !== null && (!Number.isSafeInteger(targetStep) || targetStep < 0)) {
    return null;
  }
  return {
    version: 1,
    seed,
    mode,
    scenario,
    actions,
    target_step: targetStep,
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

function normalizeApiBase(value) {
  return String(value ?? "").trim().replace(/\/+$/, "");
}

function isWholeNumber(value) {
  const text = String(value ?? "").trim();
  if (!/^\d+$/.test(text)) {
    return false;
  }
  return Number.isSafeInteger(Number.parseInt(text, 10));
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
