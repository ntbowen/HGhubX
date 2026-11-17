import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;
const { version } = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
) as { version: string };

export default defineConfig(({ mode }) => {
  // Android TV 构建期指定 VITE_APP_EDITION=tv（scripts/build-android-tv.mjs 注入）；
  // 其余情况（含未设置）维持现有标准入口 src/main.tsx，构建行为不变。
  const edition =
    process.env.VITE_APP_EDITION ||
    loadEnv(mode, process.cwd(), "VITE_").VITE_APP_EDITION;
  return {
    resolve: {
      alias: {
        "@edition-bootstrap": fileURLToPath(
          new URL(
            edition === "tv" ? "./src/tv/bootstrap.tsx" : "./src/main.tsx",
            import.meta.url,
          ),
        ),
      },
    },
    plugins: [react(), tailwindcss()],
    clearScreen: false,
    // 版本号只维护 package.json（CI 会按 tag 改写它），界面里的版本由这里注入，避免写死后漂移
    define: {
      __APP_VERSION__: JSON.stringify(version),
    },
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
      watch: { ignored: ["**/src-tauri/**"] },
    },
    envPrefix: ["VITE_", "TAURI_ENV_*"],
    build: {
      target: "es2021",
      minify: "esbuild",
      sourcemap: false,
    },
  };
});
