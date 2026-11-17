// 构建期按 edition 解析入口：标准版 src/main.tsx，TV 版 src/tv/bootstrap.tsx。
// 不动 src/main.tsx，保证标准版构建行为不变；TV 的 WebView polyfill 由
// src/tv/bootstrap.tsx 自行引入（legacyRuntime）。
void import("@edition-bootstrap");
