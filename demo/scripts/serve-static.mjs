import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const demoRoot = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const host = "127.0.0.1";
const port = Number.parseInt(process.env.HYDRACACHE_DEMO_PORT ?? "5173", 10);

const contentTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".wasm", "application/wasm"]
]);

createServer((request, response) => {
  const pathname = new URL(request.url ?? "/", `http://${host}:${port}`).pathname;
  const file = resolvePath(pathname);
  if (!file) {
    response.writeHead(404);
    response.end("not found");
    return;
  }

  try {
    const stat = statSync(file);
    if (!stat.isFile()) {
      response.writeHead(404);
      response.end("not found");
      return;
    }
    response.writeHead(200, {
      "content-type": contentTypes.get(extname(file)) ?? "application/octet-stream"
    });
    createReadStream(file).pipe(response);
  } catch (_error) {
    response.writeHead(404);
    response.end("not found");
  }
}).listen(port, host, () => {
  console.log(`HydraCache demo served from ${demoRoot} at http://${host}:${port}/demo/`);
});

function resolvePath(pathname) {
  const decoded = decodeURIComponent(pathname);
  const normalizedPath = normalize(decoded === "/" ? "/demo/" : decoded);
  const relative = normalizedPath.replace(/^[/\\]+/, "");
  const target = normalizedPath.endsWith("/")
    ? join(demoRoot, relative, "index.html")
    : join(demoRoot, relative);
  const resolved = resolve(target);
  return resolved.startsWith(demoRoot) ? resolved : null;
}
