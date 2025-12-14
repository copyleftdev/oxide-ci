-- Runs table
CREATE TABLE IF NOT EXISTS runs (
    id UUID PRIMARY KEY,
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    run_number INTEGER NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'queued',
    trigger JSONB NOT NULL,
    git_ref VARCHAR(255),
    git_sha VARCHAR(40),
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    queued_by VARCHAR(255),
    agent_id UUID,
    cancel_reason JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_runs_pipeline_run_number ON runs(pipeline_id, run_number);
CREATE INDEX idx_runs_pipeline_status ON runs(pipeline_id, status);
CREATE INDEX idx_runs_status ON runs(status);
CREATE INDEX idx_runs_queued ON runs(status, queued_at) WHERE status = 'queued';
CREATE INDEX idx_runs_created_at ON runs(created_at DESC);

-- Stages table
CREATE TABLE IF NOT EXISTS stages (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    stage_index INTEGER NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stages_run_id ON stages(run_id);
CREATE INDEX idx_stages_run_status ON stages(run_id, status);

-- Steps table
CREATE TABLE IF NOT EXISTS steps (
    id UUID PRIMARY KEY,
    stage_id UUID NOT NULL REFERENCES stages(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    step_index INTEGER NOT NULL,
    plugin VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    exit_code INTEGER,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_steps_stage_id ON steps(stage_id);
CREATE INDEX idx_steps_status ON steps(status);
