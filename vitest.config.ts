import { defineConfig } from "vitest/config";

/**
 * Vitest 配置。
 *
 * 环境：jsdom（datetime.ts/colors.ts 依赖 localStorage/document）。
 * 不处理 src-tauri 目录。
 */
export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    exclude: ["src-tauri/**", "node_modules/**"],
  },
  resolve: {
    alias: {
      "@": "/src",
    },
  },
});
