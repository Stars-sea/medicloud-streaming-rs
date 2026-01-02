FROM rust:slim AS builder

RUN sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources && \
    sed -i 's/bookworm/trixie/g' /etc/apt/sources.list.d/debian.sources

RUN apt-get update && apt-get dist-upgrade -y && apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    pkg-config \
    libssl-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Configure Cargo mirrors
ENV RUSTUP_DIST_SERVER=https://mirrors.tuna.tsinghua.edu.cn/rustup
ENV RUSTUP_UPDATE_ROOT=https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup

RUN mkdir -vp ${CARGO_HOME:-$HOME/.cargo} \
    && cat << TOML | tee -a ${CARGO_HOME:-$HOME/.cargo}/config.toml
[source.crates-io]
replace-with = 'ustc'
[source.mirror]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
[registries.mirror]
index = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"
[registries.ustc]
index = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"
TOML

WORKDIR /app
COPY . ./

# Build with release profile (dynamic linking)
RUN cargo build --release && \
    strip target/release/medicloud-streaming-rs

FROM debian:trixie-slim

RUN sed -i 's/deb.debian.org/mirrors.ustc.edu.cn/g' /etc/apt/sources.list.d/debian.sources

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/medicloud-streaming-rs ./
COPY --from=builder /app/settings.json ./

ENV GRPC_PORT=50051
ENV MINIO_ENDPOINT=http://localhost:9000
ENV MINIO_ACCESS_KEY=minioadmin
ENV MINIO_SECRET_KEY=miniokey
ENV MINIO_BUCKET=videos
ENV RUST_LOG=info

RUN mkdir ./cache

ENTRYPOINT ["./medicloud-streaming-rs"]
