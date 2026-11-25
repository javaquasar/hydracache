// Shared by the smoke and accessibility specs. The real checker does not
// violate on ordinary seeds, so this stubs the /sim server API with real WASM
// snapshots that have a violated verdict spliced in from step 9 onwards.
export const VIOLATION_API = "http://127.0.0.1:3999";

export function violatingServerUrl(demoUrl) {
  return `${demoUrl}?engine=server&api=${encodeURIComponent(VIOLATION_API)}`;
}

export async function stubViolatingServer(page, demoUrl) {
  await page.goto(demoUrl);
  const real = await page.evaluate(async () => {
    const wasm = await import(new URL("./pkg/hydracache_sim_wasm.js", window.location.href).href);
    if (typeof wasm.default === "function") {
      await wasm.default();
    }
    const handle = new wasm.SimHandle(80n);
    handle.run(8n);
    return { snapshot: JSON.parse(handle.snapshot_json()), script: JSON.parse(handle.replay_script_json()) };
  });
  const holding = { ...real.snapshot, step: 0, first_violation_step: null, verdict: { status: "holding" } };
  const ids = real.snapshot.nodes.map((node) => node.id);
  const violated = (step) => ({
    ...real.snapshot,
    step,
    first_violation_step: 9,
    verdict: {
      status: "violated",
      invariant: "election_safety",
      detail: `two leaders in term 1: ${ids[0]} and ${ids[2]}`,
    },
  });
  const controlTargets = [];
  let current = holding;
  const api = VIOLATION_API;
  await page.route(`${api}/**`, async (route) => {
    const request = route.request();
    const headers = {
      "access-control-allow-origin": "*",
      "access-control-allow-headers": "content-type",
      "access-control-allow-methods": "GET, POST, OPTIONS",
    };
    if (request.method() === "OPTIONS") {
      await route.fulfill({ status: 204, headers });
      return;
    }
    const path = new URL(request.url()).pathname;
    if (path === "/sim/step") {
      current = violated(12);
    } else if (path === "/sim/control") {
      const body = request.postDataJSON();
      controlTargets.push(body.target_step);
      current = body.target_step >= 9 ? violated(body.target_step) : { ...holding, step: body.target_step };
    } else if (path === "/sim/new") {
      current = holding;
    }
    const body = path === "/sim/replay" ? { ...real.script, actions: [] } : current;
    await route.fulfill({ status: 200, headers: { ...headers, "content-type": "application/json" }, json: body });
  });

  return { ids, controlTargets };
}
