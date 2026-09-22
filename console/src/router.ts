export const MANAGEMENT_ROUTES = Object.freeze([
  ["dashboard", "Dashboard"],
  ["healthchecks", "Healthchecks"],
  ["formation", "Formation"],
  ["members", "Members"],
  ["partitions", "Partitions"],
  ["clients", "Clients"],
  ["namespaces", "Namespaces"],
  ["replication", "Replication"],
  ["placement", "Placement"],
  ["consensus", "Consensus"],
  ["persistence", "Persistence"],
  ["operations", "Operations"],
  ["audit", "Audit"],
  ["recovery", "Recovery"],
  ["history", "Local history"],
] as const);

export type ManagementRoute = (typeof MANAGEMENT_ROUTES)[number][0];

export function routeFromHash(hash: string): ManagementRoute {
  const candidate = hash.replace(/^#/, "");
  return MANAGEMENT_ROUTES.some(([route]) => route === candidate)
    ? (candidate as ManagementRoute)
    : "dashboard";
}
