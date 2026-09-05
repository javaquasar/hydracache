// @vitest-environment happy-dom
import { render } from "preact";
import { afterEach, expect, it } from "vitest";
import { AppShell } from "./shell";

afterEach(() => document.body.replaceChildren());

it("renders all capability-driven read-only sections with bounded tables", () => {
  const root = document.createElement("div");
  document.body.append(root);
  render(<AppShell />, root);
  expect(root.querySelectorAll("nav a")).toHaveLength(15);
  expect(root.querySelectorAll("button")).toHaveLength(0);
  expect(root.querySelector("[data-testid='readonly-badge']")?.textContent).toMatch(/read only/i);
  for (const id of ["dashboard", "healthchecks", "members", "partitions", "clients", "namespaces", "persistence", "operations", "audit", "recovery", "placement"]) {
    expect(root.querySelector(`#${id}`), id).not.toBeNull();
  }
  expect([...root.querySelectorAll(".table-scroll")].every((node) => node.getAttribute("tabindex") === "0")).toBe(true);
});
