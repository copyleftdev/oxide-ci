# One Dockerfile, one compile, three images.
#
# The services previously had a Dockerfile each, which meant `docker compose
# build` compiled the whole workspace three times over. They share a builder
# stage now and select a runtime stage with `target:`.
#
# Dependencies are compiled in their own layer via cargo-chef, so editing a
# crate rebuilds that crate rather than the ~600 crates underneath it. The
# layer is invalidated only by Cargo.toml or Cargo.lock changing, which is what
# actually changes the dependency graph.
#
# Debian rather than Alpine: the workspace links ring and sqlx, and musl builds
# of those are a reliable way to lose an afternoon. The image is larger and it
# builds the first time.

FROM rust:1.90-slim-bookworm AS chef

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Pinned, like every other tag here: an unpinned tool is a build that changes
# under you for reasons unrelated to your code.
RUN cargo install cargo-chef --version 0.1.77 --locked

WORKDIR /app

# ---------------------------------------------------------------------------
# Reduce the workspace to a dependency recipe. This stage sees the source but
# produces only recipe.json, so source edits cannot invalidate the layer below.

FROM chef AS planner

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------

FROM chef AS builder

# Dependencies only. Cached until the dependency graph itself changes.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Now the workspace's own code, which is all that recompiles on an edit.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
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
