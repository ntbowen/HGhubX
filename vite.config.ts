import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";

const host = process.env.TAURI_DEV_HOST;
const { version } = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
) as { version: string };

export default defineConfig({
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
  css: {
    // 老 WebView（机顶盒/车机常见 Chromium 80+）不支持 inset、aspect-ratio、
    // color-mix、dvh 等新特性；lightningcss 按目标版本自动降级为兼容写法，
    // esbuild minify 只压缩不转译，挡不住 Tailwind inset-0 这类工具类
    transformer: "lightningcss",
    lightningcss: { targets: { chrome: 83 << 16 } },
  },
  build: {
    // 机顶盒/车机的系统 WebView 常停留在 Chromium 80+；es2021 的逻辑赋值符
    // 需要 Chrome 85，降到 es2020 保证老 WebView 可解析（现代浏览器不受影响）
    target: "es2020",
    // 同理约束 CSS 压缩目标：否则 esbuild minify 会把 top/right/bottom/left
    // 长手合并回 Chrome 87 才有的 inset 简写，老 WebView 直接丢声明
    cssTarget: "chrome83",
    cssMinify: "lightningcss",
    minify: "esbuild",
    sourcemap: false,
  },
});
