import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      exclude: ["src/controller.ts", "src/**/*.test.{ts,tsx}"],
      include: ["src/api.ts", "src/history.ts", "src/router.ts", "src/state.ts", "src/components/**/*.tsx"],
      provider: "v8",
      reporter: ["text", "json-summary"],
      reportsDirectory: "../target/test-evidence/0.72/console-coverage",
      thresholds: { lines: 88, statements: 88, functions: 88, branches: 80 },
    },
  },
});
