// @vitest-environment happy-dom
import { render } from "preact";
import { act } from "preact/test-utils";
import { afterEach, expect, it } from "vitest";
import { AppShell } from "./shell";
import { ConsoleErrorBoundary } from "./error-boundary";

afterEach(() => document.body.replaceChildren());

it("renders all capability-driven read-only sections with bounded tables", () => {
  const root = document.createElement("div");
  document.body.append(root);
  render(<AppShell />, root);
  expect(root.querySelectorAll("nav a")).toHaveLength(15);
  expect(root.querySelectorAll("button")).toHaveLength(0);
  expect(root.querySelector("[data-testid='cluster-selector']")?.hasAttribute("disabled")).toBe(true);
  expect(root.querySelector("[data-testid='readonly-badge']")?.textContent).toMatch(/read only/i);
  for (const id of ["dashboard", "healthchecks", "members", "partitions", "clients", "namespaces", "persistence", "operations", "audit", "recovery", "placement"]) {
    expect(root.querySelector(`#${id}`), id).not.toBeNull();
  }
  expect([...root.querySelectorAll(".table-scroll")].every((node) => node.getAttribute("tabindex") === "0")).toBe(true);
});

it("redacts component render exceptions behind a stable error boundary", () => {
  const root = document.createElement("div");
  document.body.append(root);
  const Broken = () => {
    throw new Error("secret-token-from-render");
  };
  act(() => render(<ConsoleErrorBoundary><Broken /></ConsoleErrorBoundary>, root));
  expect(root.querySelector("[role='alert']")?.textContent).toContain("could not render trusted data");
  expect(root.textContent).not.toContain("secret-token-from-render");
});
