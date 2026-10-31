# 构建阶段
FROM rust:1.83-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY migrations ./migrations
COPY tests ./tests
RUN cargo build --release --bin ai-workforce

# 运行阶段：单二进制 + ffmpeg（视频工具）+ 证书
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/ai-workforce /app/ai-workforce
COPY migrations /app/migrations
ENV AWF_MEDIA_DIR=/app/data/media
EXPOSE 8600
CMD ["/app/ai-workforce"]
