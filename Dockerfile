# syntax=docker/dockerfile:1.4
###############################################################################
# Multi-arch Rust builder for reproducible WASM contracts
#
#   docker buildx build --platform linux/amd64  -t rust-wasm-amd  --load .
#   docker buildx build --platform linux/arm64/v8 -t rust-wasm-arm  --load .
#
# Объектный код собирается в каталоге /workspace — одинаковом на всех платформах.
###############################################################################

ARG RUST_VERSION=1.87          # нужная версия toolchain
# базовый образ автоматически берётся под платформу, указанную в --platform
FROM rust:${RUST_VERSION}

# ───── инструменты для отладки / сравнения .wasm ─────
RUN apt-get update -qq && apt-get install -y --no-install-recommends \
        wabt  xxd  git  ca-certificates  && \
    rm -rf /var/lib/apt/lists/*

# ───── таргет wasm32 ─────
RUN rustup target add wasm32-unknown-unknown

# ───── путь фиксации сборки ─────
WORKDIR /workspace     # ← абсолютный путь, одинаковый в любом образе

# ───── точка входа ─────
ENTRYPOINT ["/bin/bash"]
