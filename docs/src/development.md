# Local Development

Two ways to run Oxide CI locally. Pick by what you are doing, not by which is
more "real".

| | `make dev-up` | `make stack-up` |
|---|---|---|
| Services run | natively, via `cargo run` | in containers |
| Infrastructure | Postgres + NATS in containers | adds MinIO and Jaeger |
| Code change costs | seconds — rebuild one binary | an image rebuild |
| Ports | 18080 / 55432 / 54222 | 8080 / 5432 / 4222 |
| Use it for | writing code | checking it works the way it ships |

The dev stack deliberately uses non-default ports so it never fights a
containerised stack you already have running.

## The fast loop

```bash
make dev-up      # Postgres, NATS, api, scheduler, agent — waits until the API answers
make dev-smoke   # creates a pipeline, runs it, proves it reached an agent
make dev-status  # what is running, and where
make dev-logs    # follow all three service logs
make dev-down    # stop everything it started
```

`make dev-up` blocks until `/health` answers, so when it returns the stack is
actually usable — not merely started.

`make dev-smoke` is the check worth running after a change to the distributed
path. It creates a pipeline, triggers a run, and waits for the agent to finish
it, then prints the three lines that prove the path end to end:

```
the run reached the agent and completed:
  Dispatching  job_id=job_019f… agent=dev-agent stage=build queue_wait_ms=241
  Accepted job job_id=job_019f… stage=build steps=1 queue_wait_ms=241
  Job finished job_id=job_019f… success=true duration_ms=3
```

If a run never reaches an agent, those lines are where the trail stops, and
which one is missing tells you which service to look at.

## The containerised stack

```bash
make stack-up     # docker compose up -d --build
make stack-logs   # follow api, scheduler and agent
make stack-down
```

All three services share one builder stage in a single `Dockerfile`, selected
with `target:`. They previously had a Dockerfile each, which compiled the whole
workspace three times for one `docker compose build`.

## What the services expect

Every setting is an environment variable, with defaults that work on a laptop.

| Variable | Used by | Default |
|---|---|---|
| `DATABASE_URL` | api, scheduler, agent | `postgres://oxide:oxide_dev_password@localhost:5432/oxide` |
| `NATS_URL` | api, scheduler, agent | `nats://localhost:4222` |
| `API_HOST` / `API_PORT` | api | `0.0.0.0` / `8080` |
| `SCHEDULER_POLL_INTERVAL_MS` | scheduler | `500` |
| `AGENT_NAME` | agent | `$HOSTNAME`, then `oxide-agent` |
| `AGENT_LABELS` | agent | none — comma separated |
| `AGENT_MAX_CONCURRENT_JOBS` | agent | `4` |
| `AGENT_WORKSPACE_DIR` | agent | platform temp directory |
| `AGENT_CONFIG` | agent | unset; a YAML file wins over the environment |

The API applies database migrations on start, so a fresh Postgres needs no
separate step.

## How a run actually flows

Worth knowing when something stalls, because each hop is a different process:

1. `POST /api/v1/pipelines/{id}/runs` creates the run and publishes `RunQueued`.
2. The scheduler **adopts** that run — it did not create it — builds the DAG,
   and queues the stages with no dependencies.
3. Each poll, the scheduler matches queued jobs to registered agents and
   publishes `JobDispatched` to `agent.{agent_id}.job.dispatched`.
4. The agent answers every dispatch: `JobAccepted`, or `JobRejected` with a
   typed reason. Silence would leave the scheduler believing a job is running
   somewhere it is not.
5. The agent runs the steps and publishes stage and step events; the scheduler
   advances the DAG on `StageCompleted`.

A run that sits in `queued` usually means step 2 or 3: no scheduler running, or
no agent whose labels match. `make dev-status` answers the first;
`GET /api/v1/agents` answers the second.

## Troubleshooting

**`make dev-up` says the API never became healthy.** Read
`.oxide-dev/oxide-api.log`. Usually Postgres is not up yet or the port is
taken.

**A run never reaches an agent.** Check `GET /api/v1/agents` — an agent that
never registered cannot be dispatched to. Then check the scheduler log for
`Adopted run`; without it, the scheduler is not seeing `RunQueued`.

**`docker compose build` is slow the first time.** It compiles the workspace
in release mode. Subsequent builds reuse the layer unless `crates/` changed.

## See also

- [Testing Strategy](testing.md) — the tiers and what may block a merge
- [System Diagrams](diagrams.md) — how the pieces fit together
