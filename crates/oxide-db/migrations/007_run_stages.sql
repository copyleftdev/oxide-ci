-- Run stages and steps.
--
-- Migration 002 created `stages` and `steps` tables keyed by `id UUID`, but
-- StageId and StepId are name-based strings ("build", "greet") — a UUID column
-- could never have held one. Nothing ever wrote to either table: no INSERT,
-- UPDATE or SELECT anywhere in the workspace names them. They are dropped
-- rather than left standing, because a table that looks authoritative and is
-- never written is the same trap as a second schema source.
--
-- Identity comes from position within a run. A stage is (run_id, stage_index)
-- and a step is (run_id, stage_index, step_index), which is also the order they
-- must be read back in.

-- step_logs referenced steps(id) and is dead for the same reason, so it is
-- rebuilt below against the new identity rather than left pointing at a table
-- that no longer exists.
DROP TABLE IF EXISTS step_logs;
DROP TABLE IF EXISTS steps;
DROP TABLE IF EXISTS stages;

CREATE TABLE IF NOT EXISTS run_stages (
    run_id UUID NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    stage_index INTEGER NOT NULL,
    stage_id TEXT NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    depends_on TEXT[] NOT NULL DEFAULT '{}',
    agent_id UUID,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    PRIMARY KEY (run_id, stage_index)
);

CREATE INDEX idx_run_stages_status ON run_stages(status);
-- "what is running right now" is the question an operator asks most.
CREATE INDEX idx_run_stages_active ON run_stages(run_id) WHERE status = 'running';

CREATE TABLE IF NOT EXISTS run_steps (
    run_id UUID NOT NULL,
    stage_index INTEGER NOT NULL,
    step_index INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    plugin TEXT,
    exit_code INTEGER,
    outputs JSONB NOT NULL DEFAULT '{}',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    PRIMARY KEY (run_id, stage_index, step_index),
    FOREIGN KEY (run_id, stage_index)
        REFERENCES run_stages(run_id, stage_index) ON DELETE CASCADE
);

CREATE INDEX idx_run_steps_run ON run_steps(run_id);

-- Step logs, rebuilt against the new step identity. Still unwritten by any
-- code, but now shaped so that when something does write it, the reference
-- means something.
CREATE TABLE IF NOT EXISTS step_logs (
    id BIGSERIAL PRIMARY KEY,
    run_id UUID NOT NULL,
    stage_index INTEGER NOT NULL,
    step_index INTEGER NOT NULL,
    stream VARCHAR(10) NOT NULL CHECK (stream IN ('stdout', 'stderr')),
    line_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (run_id, stage_index, step_index)
        REFERENCES run_steps(run_id, stage_index, step_index) ON DELETE CASCADE
);

CREATE INDEX idx_step_logs_step ON step_logs(run_id, stage_index, step_index, line_number);
