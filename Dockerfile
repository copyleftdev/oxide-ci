# One Dockerfile, one compile, three images.
#
# The services previously had a Dockerfile each, which meant `docker compose
# build` compiled the whole workspace three times over. They share a builder
# stage now and select a runtime stage with `target:`.
#
# Debian rather than Alpine: the workspace links ring and sqlx, and musl builds
# of those are a reliable way to lose an afternoon. The image is larger and it
# builds the first time.

FROM rust:1.90-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# All three binaries in one pass, so the dependency graph is built once.
RUN cargo build --release -p oxide-api -p oxide-scheduler -p oxide-agent

# ---------------------------------------------------------------------------

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /usr/sbin/nologin oxide

WORKDIR /app
ENV RUST_LOG=info,oxide=debug

# ---------------------------------------------------------------------------

FROM runtime AS api
COPY --from=builder /app/target/release/oxide-api /usr/local/bin/oxide-api
USER oxide
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=20s --retries=5 \
    CMD curl -fsS http://localhost:8080/health || exit 1
CMD ["oxide-api"]

# ---------------------------------------------------------------------------

FROM runtime AS scheduler
COPY --from=builder /app/target/release/oxide-scheduler /usr/local/bin/oxide-scheduler
USER oxide
CMD ["oxide-scheduler"]

# ---------------------------------------------------------------------------

FROM runtime AS agent
# The agent runs jobs in containers, so it needs the workspace it mounts and
# the docker socket the compose file gives it. It stays root for that reason.
COPY --from=builder /app/target/release/oxide-agent /usr/local/bin/oxide-agent
ENV AGENT_WORKSPACE_DIR=/var/oxide/workspace
RUN mkdir -p /var/oxide/workspace
CMD ["oxide-agent"]
