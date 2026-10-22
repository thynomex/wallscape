import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [svelte({ configFile: false })],
  resolve: {
    alias: { $lib: fileURLToPath(new URL("./src/lib", import.meta.url)) },
    conditions: ["browser"],
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
    clearMocks: true,
    restoreMocks: true,
  },
});
