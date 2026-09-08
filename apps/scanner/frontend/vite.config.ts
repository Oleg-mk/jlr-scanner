import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { defineConfig } from "vitest/config";

// The build's identity, shown in the footer and written into every report:
// the version from package.json, and the commit CI built (a local build
// says "local"). Kept with the code, so a tester's screenshot names the build.
const version = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")).version as string;
const sha = (process.env.VITE_BUILD_SHA ?? process.env.GITHUB_SHA ?? "local").slice(0, 7);

export default defineConfig({
  plugins: [react()],
  define: {
    __APP_VERSION__: JSON.stringify(version),
    __BUILD_SHA__: JSON.stringify(sha),
  },
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
  },
});
