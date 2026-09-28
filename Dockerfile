# ============================================================
# HotDownloader — 单镜像（Rust 服务同时托管 Web 页面与 API）
#
# 多阶段构建：
#   1) web-builder  构建前端产物（dist）
#   2) rust-builder 编译统一服务（crates/hotdownloader-server）
#   3) runtime      仅保留 Rust 二进制 + dist，单进程同时提供页面与 API
#
# 说明：本文件基于上游 v2.0.0 的 Dockerfile，保留其单镜像方案。
# ============================================================

# ---- 阶段 1：构建前端 ----
FROM node:24-trixie-slim AS web-builder
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY index.html vite.config.ts tsconfig*.json ./
COPY public ./public
COPY src ./src
COPY src-tauri/tauri.conf.json ./src-tauri/tauri.conf.json
RUN npm run build

# ---- 阶段 2：编译 Rust 服务（共享核心 + 加密库）----
FROM rust:1-trixie AS rust-builder
WORKDIR /build
COPY crates ./crates
COPY libs/um_crypto ./libs/um_crypto
RUN cargo build --release --locked --manifest-path crates/hotdownloader-server/Cargo.toml

# ---- 阶段 3：运行 ----
FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /build/crates/hotdownloader-server/target/release/hotdownloader-server /app/hotdownloader-server
COPY --from=web-builder /build/dist /app/dist
COPY LICENSE /app/LICENSE
COPY crates/hotdownloader-server/NOTICE /app/NOTICE
COPY crates/hotdownloader-server/THIRD_PARTY_LICENSES.txt /app/THIRD_PARTY_LICENSES.txt

# 数据目录保存任务、凭据和设置；下载目录可通过环境变量独立指定并挂载。
ENV HOTDOWNLOADER_BIND=0.0.0.0:8787 \
    HOTDOWNLOADER_DATA_DIR=/data \
    HOTDOWNLOADER_WEB_DIR=/app/dist
VOLUME /data
EXPOSE 8787

# 真实健康检查：服务能响应 /healthz 即表示 Web+API 正常。
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl --fail --silent --show-error http://127.0.0.1:8787/healthz || exit 1

CMD ["/app/hotdownloader-server"]
