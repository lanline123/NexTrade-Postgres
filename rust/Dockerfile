# NexTrade Rust 构建与运行
#
# 对应原 Java 的 build-and-run.sh / docker-compose.yml

# ---------- 构建 ----------
# cargo build --release          # 生产构建
# cargo test                     # 单元测试
# cargo clippy --workspace       # 静态检查
# cargo fmt                      # 格式化

# ---------- Docker ----------
# docker compose up -d --build   # 启动 mysql + redis + rabbitmq + 应用

FROM rust:bookworm AS builder
WORKDIR /app

# 先拷贝清单以利用层缓存
COPY crates/Cargo.toml crates/rust-toolchain.toml ./crates/
COPY crates/core/Cargo.toml ./crates/core/
COPY crates/common/Cargo.toml ./crates/common/
COPY crates/infrastructure/Cargo.toml ./crates/infrastructure/
COPY crates/security/Cargo.toml ./crates/security/
COPY crates/services/Cargo.toml ./crates/services/
COPY crates/api/admin/Cargo.toml ./crates/api/admin/
COPY crates/api/public/Cargo.toml ./crates/api/public/
COPY crates/api/openapi/Cargo.toml ./crates/api/openapi/
COPY crates/bin/Cargo.toml ./crates/bin/

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev default-libmysqlclient-dev \
    && rm -rf /var/lib/apt/lists/*

# 预下载依赖（源码未就位时先构建空壳会失败，故直接全量拷贝后构建）
COPY crates ./crates
RUN cargo build --release --manifest-path crates/Cargo.toml -p nextrade-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 default-mysql-client curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/crates/target/release/nextrade /usr/local/bin/nextrade
COPY config.yaml ./config.yaml
COPY migrations ./migrations
COPY template ./template

ENV RUST_LOG=info,nextrade=info
EXPOSE 8080 8181

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
    CMD curl -fs http://127.0.0.1:8080/adminapi/health || exit 1

ENTRYPOINT ["nextrade"]
