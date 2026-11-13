import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/setupTests.ts",
    globals: true,
    coverage: {
      provider: "v8",
      reporter: ["text", "json-summary", "lcov"],
      // Include every source file so untested pages are visible too.
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/**/*.test.{ts,tsx}",
        "src/setupTests.ts",
        "src/types/**",
        "src/i18n/locales/**",
        "src/vite-env.d.ts",
      ],
      // Initial "no-regression" gate measured on 2026-11-13 with all source
      // files included (7.61% stmts / 7.6% lines / 8.41% funcs / 3.42% branches).
      // Raise these by ~5 points per iteration as page/component tests land.
      thresholds: {
        statements: 7,
        lines: 7,
        functions: 8,
        branches: 3,
      },
    },
  },
});
