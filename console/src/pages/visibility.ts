import type { ManagementRoute } from "../router";

export function panelIsVisible(
  activeRoute: ManagementRoute,
  panelRoute: string,
  capabilityAvailable: boolean,
): boolean {
  if (!capabilityAvailable) return false;
  return activeRoute === "dashboard" || panelRoute === activeRoute;
}
