import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const allowedLicenses = new Set([
  "Apache-2.0",
  "BlueOak-1.0.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "ISC",
  "MIT",
  "MPL-2.0",
]);

if (lock.lockfileVersion !== 3 || !lock.requires) {
  throw new Error("management console requires npm lockfileVersion 3");
}
for (const [name, version] of Object.entries({
  ...(manifest.dependencies ?? {}),
  ...(manifest.devDependencies ?? {}),
})) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`direct dependency is not exact: ${name}@${version}`);
  }
}

const components = [];
for (const [path, pkg] of Object.entries(lock.packages ?? {})) {
  if (!path.startsWith("node_modules/")) continue;
  const name = path.slice("node_modules/".length);
  if (!pkg.version || !pkg.resolved?.startsWith("https://registry.npmjs.org/") || !pkg.integrity) {
    throw new Error(`dependency provenance is incomplete: ${name}`);
  }
  if (!allowedLicenses.has(pkg.license)) {
    throw new Error(`dependency license is not reviewed: ${name} (${pkg.license ?? "missing"})`);
  }
  components.push({
    type: "library",
    name,
    version: pkg.version,
    scope: pkg.dev ? "optional" : "required",
    purl: `pkg:npm/${encodeURIComponent(name)}@${pkg.version}`,
    externalReferences: [{ type: "distribution", url: pkg.resolved }],
    properties: [
      { name: "npm:integrity", value: pkg.integrity },
      { name: "npm:license", value: pkg.license },
    ],
  });
}
components.sort((left, right) => left.name.localeCompare(right.name));

const sbom = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  serialNumber: "urn:uuid:00000000-0000-4000-8000-000000000072",
  version: 1,
  metadata: {
    component: {
      type: "application",
      name: manifest.name,
      version: manifest.version,
    },
  },
  components,
};
const destination = join(root, "../target/management-center-0.72-sbom.cdx.json");
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, `${JSON.stringify(sbom, null, 2)}\n`, "utf8");
console.log(`supply-chain checks passed; wrote ${components.length} components to ${destination}`);
