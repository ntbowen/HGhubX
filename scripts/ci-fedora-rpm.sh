#!/usr/bin/env bash
# 在 Fedora 容器里构建 HGhubX RPM。
# 由 .github/workflows/release.yml 的 linux-fedora job 通过
#   docker run --rm -v "$GITHUB_WORKSPACE:/src" -w /src fedora:45 bash scripts/ci-fedora-rpm.sh
# 调用；版本号对齐已在宿主机上由 scripts/set-version.mjs 完成。
set -euo pipefail

# ---- 系统依赖 ----
# webkit2gtk4.1-devel 是 Tauri 2 在 Linux 上的核心依赖；rpm-build 提供 rpmbuild；
# appstream 用于校验生成的 metainfo；其余与 Ubuntu CI 的清单对齐。
dnf install -y --setopt=install_weak_deps=False \
  webkit2gtk4.1-devel gtk3-devel librsvg2-devel \
  patchelf openssl-devel libxdo-devel \
  rpm-build appstream file \
  gcc make pkgconf-pkg-config \
  nodejs npm curl

# ---- Rust（rustup stable，满足 Cargo.toml 的 rust-version = "1.90"）----
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
# shellcheck disable=SC1091
. "$HOME/.cargo/env"

# ---- 前端 ----
npm install -g pnpm@10
pnpm install --frozen-lockfile

# ---- 构建 RPM（目标取自 src-tauri/tauri.linux.conf.json + --bundles rpm）----
pnpm tauri build --bundles rpm
