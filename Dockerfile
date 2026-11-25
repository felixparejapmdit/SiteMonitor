# Build context: the folder produced by python deploy-to-nas.py --prepare-only.
# Dioxus/WebAssembly is built on the workstation and supplied in web/.
ARG RUST_IMAGE=rust:1.99.1-slim-bookworm
FROM ${RUST_IMAGE} AS builder
WORKDIR /build

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Keep the NAS build within its memory budget. Only the native backend is built.
ENV CARGO_BUILD_JOBS=1 \
    CARGO_PROFILE_RELEASE_LTO=false \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/Cargo.toml
COPY backend/src backend/src
COPY shared/Cargo.toml shared/Cargo.toml
COPY shared/src shared/src
# Cargo resolves every workspace member, including the unbuilt web frontend.
COPY frontend/Cargo.toml frontend/Cargo.toml
COPY frontend/src frontend/src
RUN cargo build --locked --release -p backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates curl gosu tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 sitemonitor \
    && useradd --uid 10001 --gid sitemonitor --no-create-home --shell /usr/sbin/nologin sitemonitor
WORKDIR /app
COPY --from=builder /build/target/release/backend /usr/local/bin/backend
COPY web/ /app/public/
COPY docker/entrypoint.sh /usr/local/bin/sitemonitor-entrypoint
RUN chmod 755 /usr/local/bin/sitemonitor-entrypoint \
    && test -s /app/public/index.html \
    && mkdir -p /app/data/uploads /app/data/keys \
    && chown -R sitemonitor:sitemonitor /app/data

ENV TZ=Asia/Singapore \
    SITEMONITOR_PRODUCTION=1 \
    SITEMONITOR_DEMO=0 \
    PORT=8737 \
    STATIC_DIR=/app/public \
    UPLOAD_DIR=/app/data/uploads \
    DATA_KEY_FILE=/app/data/keys/personnel.key \
    RUST_LOG=backend=info,tower_http=warn,sea_orm=warn,sqlx=warn
EXPOSE 8737
HEALTHCHECK --interval=30s --timeout=8s --start-period=60s --retries=3 \
    CMD curl --fail --silent --show-error http://127.0.0.1:8737/api/ready || exit 1
ENTRYPOINT ["/usr/local/bin/sitemonitor-entrypoint"]
CMD ["backend", "serve"]
