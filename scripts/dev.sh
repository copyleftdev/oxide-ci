#!/usr/bin/env bash
# Local development stack.
#
# Runs Postgres, NATS and MinIO in containers and the three Oxide services
# natively, because rebuilding an image to test a one-line change is not a
# development loop. `cargo run` picks up your edit in seconds.
#
#   ./scripts/dev.sh up       infra + services, wait until the API answers
#   ./scripts/dev.sh down     stop everything this script started
#   ./scripts/dev.sh logs     follow the service logs
#   ./scripts/dev.sh status   what is running, and where
#   ./scripts/dev.sh smoke    create a pipeline, run it, show what happened
#
# Ports are deliberately not the defaults, so this never fights a stack you
# already have up via docker compose.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

RUN_DIR="${OXIDE_DEV_DIR:-$ROOT/.oxide-dev}"
PG_PORT="${OXIDE_DEV_PG_PORT:-55432}"
NATS_PORT="${OXIDE_DEV_NATS_PORT:-54222}"
API_PORT="${OXIDE_DEV_API_PORT:-18080}"
API_URL="http://localhost:$API_PORT"

export DATABASE_URL="postgres://oxide:oxide_dev_password@localhost:$PG_PORT/oxide"
export NATS_URL="nats://localhost:$NATS_PORT"

green() { printf "\033[32m%s\033[0m\n" "$*"; }
dim()   { printf "\033[2m%s\033[0m\n" "$*"; }
fail()  { printf "\033[31m%s\033[0m\n" "$*" >&2; }

require_docker() {
    if ! docker info >/dev/null 2>&1; then
        fail "Docker is not running — the dev stack needs Postgres and NATS."
        exit 1
    fi
}

start_infra() {
    docker rm -f oxide-dev-pg oxide-dev-nats >/dev/null 2>&1
    docker run -d --name oxide-dev-pg \
        -e POSTGRES_USER=oxide -e POSTGRES_PASSWORD=oxide_dev_password -e POSTGRES_DB=oxide \
        -p "$PG_PORT:5432" postgres:16-alpine >/dev/null || { fail "Could not start Postgres"; exit 1; }
    docker run -d --name oxide-dev-nats \
        -p "$NATS_PORT:4222" nats:2.10-alpine -js >/dev/null || { fail "Could not start NATS"; exit 1; }

    printf "waiting for Postgres "
    for _ in $(seq 1 60); do
        if docker exec oxide-dev-pg pg_isready -U oxide >/dev/null 2>&1; then
            echo; return 0
        fi
        printf "."; sleep 1
    done
    echo; fail "Postgres never became ready"; exit 1
}

start_service() {
    local name="$1"; shift
    ( env "$@" cargo run --quiet --bin "$name" > "$RUN_DIR/$name.log" 2>&1 & echo $! > "$RUN_DIR/$name.pid" )
}

cmd_up() {
    require_docker
    mkdir -p "$RUN_DIR"

    dim "building services (first run compiles the workspace)"
    if ! cargo build --quiet --bin oxide-api --bin oxide-scheduler --bin oxide-agent; then
        fail "build failed — fix that first"; exit 1
    fi

    start_infra

    start_service oxide-api API_PORT="$API_PORT"
    printf "waiting for the API "
    for _ in $(seq 1 60); do
        if curl -sf "$API_URL/health" >/dev/null 2>&1; then echo; break; fi
        printf "."; sleep 1
    done
    if ! curl -sf "$API_URL/health" >/dev/null 2>&1; then
        echo; fail "the API never became healthy — see $RUN_DIR/oxide-api.log"; exit 1
    fi

    start_service oxide-scheduler SCHEDULER_POLL_INTERVAL_MS=300
    start_service oxide-agent AGENT_NAME=dev-agent AGENT_LABELS=linux,docker \
        AGENT_WORKSPACE_DIR="$RUN_DIR/workspace"

    green "dev stack is up"
    echo "  API        $API_URL"
    echo "  Postgres   localhost:$PG_PORT"
    echo "  NATS       localhost:$NATS_PORT"
    echo "  logs       $RUN_DIR/*.log   (make dev-logs)"
    echo "  smoke test make dev-smoke"
}

cmd_down() {
    for name in oxide-agent oxide-scheduler oxide-api; do
        if [ -f "$RUN_DIR/$name.pid" ]; then
            kill -TERM "$(cat "$RUN_DIR/$name.pid")" 2>/dev/null
            rm -f "$RUN_DIR/$name.pid"
        fi
    done
    # cargo run wraps the binary, so the child can outlive the pid we recorded.
    pkill -f 'target/debug/oxide-(api|scheduler|agent)' 2>/dev/null
    docker rm -f oxide-dev-pg oxide-dev-nats >/dev/null 2>&1
    green "dev stack is down"
}

cmd_logs() {
    if ! ls "$RUN_DIR"/*.log >/dev/null 2>&1; then
        fail "no logs yet — run 'make dev-up' first"; exit 1
    fi
    tail -f "$RUN_DIR"/*.log
}

cmd_status() {
    printf "%-12s %s\n" "component" "state"
    for name in oxide-api oxide-scheduler oxide-agent; do
        if [ -f "$RUN_DIR/$name.pid" ] && ps -p "$(cat "$RUN_DIR/$name.pid")" >/dev/null 2>&1; then
            printf "%-12s running\n" "${name#oxide-}"
        else
            printf "%-12s stopped\n" "${name#oxide-}"
        fi
    done
    for c in oxide-dev-pg oxide-dev-nats; do
        state=$(docker inspect -f '{{.State.Status}}' "$c" 2>/dev/null || echo "absent")
        printf "%-12s %s\n" "${c#oxide-dev-}" "$state"
    done
    curl -sf "$API_URL/health" >/dev/null 2>&1 && green "API is healthy at $API_URL" || dim "API is not answering"
}

# End to end through the running stack: create, trigger, and report what the
# services actually did. This is the check that the whole path works, not just
# that processes are alive.
cmd_smoke() {
    if ! curl -sf "$API_URL/health" >/dev/null 2>&1; then
        fail "the API is not answering — run 'make dev-up' first"; exit 1
    fi

    local name="smoke-$(date +%s 2>/dev/null || echo run)"
    local created
    created=$(curl -sf -X POST "$API_URL/api/v1/pipelines" -H 'Content-Type: application/json' \
        -d "{\"version\":\"1\",\"name\":\"$name\",\"stages\":[{\"name\":\"build\",\"steps\":[{\"name\":\"greet\",\"run\":\"echo hello from oxide\"}]}]}")
    if [ -z "$created" ]; then fail "could not create a pipeline"; exit 1; fi

    local pipeline_id
    pipeline_id=$(printf '%s' "$created" | sed -n 's/.*"id":"pip_\([^"]*\)".*/\1/p')
    green "created pipeline $name"

    curl -sf -X POST "$API_URL/api/v1/pipelines/$pipeline_id/runs" \
        -H 'Content-Type: application/json' -d '{}' >/dev/null || { fail "could not trigger a run"; exit 1; }
    green "triggered a run"

    printf "waiting for the agent to finish it "
    for _ in $(seq 1 30); do
        if grep -q "Job finished" "$RUN_DIR/oxide-agent.log" 2>/dev/null; then
            echo
            green "the run reached the agent and completed:"
            grep -h "Dispatching" "$RUN_DIR/oxide-scheduler.log" | tail -1 | sed 's/^/  /'
            grep -hE "Accepted job|Job finished" "$RUN_DIR/oxide-agent.log" | tail -2 | sed 's/^/  /'
            return 0
        fi
        printf "."; sleep 1
    done
    echo
    fail "no job completed within 30s — check $RUN_DIR/oxide-scheduler.log and oxide-agent.log"
    exit 1
}

case "${1:-}" in
    up)     cmd_up ;;
    down)   cmd_down ;;
    logs)   cmd_logs ;;
    status) cmd_status ;;
    smoke)  cmd_smoke ;;
    *)
        echo "usage: $0 {up|down|logs|status|smoke}"
        exit 64
        ;;
esac
