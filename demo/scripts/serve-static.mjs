import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, resolve, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

// Local development server for the simulator demo. It serves only the `demo/`
// directory, at both `/demo/` and the GitHub Pages path `/hydracache/demo/`, so
// a local run sees the same relative-URL layout as the published site.
export const demoDir = resolve(fileURLToPath(new URL("..", import.meta.url)));
const host = "127.0.0.1";
const MOUNTS = ["/demo/", "/hydracache/demo/"];
const HIDDEN_SEGMENTS = new Set(["node_modules", "test-results", "playwright-report"]);

const contentTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".mjs", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".wasm", "application/wasm"],
]);

// Map a request pathname to a file inside `root`, or explain why not.
// Returns { file } for a servable path, { redirect } for a mount root without
// its trailing slash, or { status } for a rejected path. Never throws.
export function resolveRequestPath(pathname, root = demoDir) {
  let decoded;
  try {
    decoded = decodeURIComponent(pathname);
  } catch (_error) {
    return { status: 400 };
  }
  if (decoded.includes("\0") || decoded.includes("\\")) {
    return { status: 400 };
  }
  if (decoded === "/") {
    return { redirect: MOUNTS[0] };
  }
  for (const mount of MOUNTS) {
    if (decoded === mount.slice(0, -1)) {
      return { redirect: mount };
    }
  }
  const mount = MOUNTS.find((candidate) => decoded.startsWith(candidate));
  if (!mount) {
    return { status: 404 };
  }
  const segments = decoded.slice(mount.length).split("/");
  if (segments.some((segment) => segment === ".." || segment.startsWith(".") || HIDDEN_SEGMENTS.has(segment))) {
    return { status: 404 };
  }
  // Test the directory form on the request path: `path.normalize` would turn
  // `/` into `\` on Windows and hide it.
  const relative = decoded.endsWith("/") ? [...segments.slice(0, -1), "index.html"] : segments;
  const file = resolve(join(root, ...relative));
  return isInsideRoot(root, file) ? { file } : { status: 404 };
}

// Containment must compare whole path segments: `/srv/demo-evil` starts with
// `/srv/demo` as a string but is not inside it.
export function isInsideRoot(root, file) {
  const base = resolve(root);
  const target = resolve(file);
  return target === base || target.startsWith(base.endsWith(sep) ? base : `${base}${sep}`);
}

function send(response, status, body) {
  response.writeHead(status, {
    "content-type": "text/plain; charset=utf-8",
    "x-content-type-options": "nosniff",
  });
  response.end(body);
}

export function startServer(port, root = demoDir) {
  const server = createServer((request, response) => {
    let pathname;
    try {
      pathname = new URL(request.url ?? "/", `http://${host}`).pathname;
    } catch (_error) {
      send(response, 400, "bad request");
      return;
    }
    const target = resolveRequestPath(pathname, root);
    if (target.redirect) {
      response.writeHead(302, { location: target.redirect });
      response.end();
      return;
    }
    if (!target.file) {
      send(response, target.status, target.status === 400 ? "bad request" : "not found");
      return;
    }
    let stat;
    try {
      stat = statSync(target.file);
    } catch (_error) {
      send(response, 404, "not found");
      return;
    }
    if (!stat.isFile()) {
      send(response, 404, "not found");
      return;
    }
    response.writeHead(200, {
      "content-type": contentTypes.get(extname(target.file)) ?? "application/octet-stream",
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
    });
    createReadStream(target.file)
      .on("error", () => response.destroy())
      .pipe(response);
  });
  return new Promise((resolveServer) => {
    server.listen(port, host, () => resolveServer(server));
  });
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const port = Number.parseInt(process.env.HYDRACACHE_DEMO_PORT ?? "5173", 10);
  const server = await startServer(port);
  const { port: actualPort } = server.address();
  console.log(
    `HydraCache demo served from ${demoDir} at http://${host}:${actualPort}/demo/ ` +
      `(Pages layout: http://${host}:${actualPort}/hydracache/demo/)`,
  );
}
