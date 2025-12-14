-- Pipelines table
CREATE TABLE IF NOT EXISTS pipelines (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    definition JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pipelines_name ON pipelines(name);
CREATE INDEX idx_pipelines_created_at ON pipelines(created_at DESC);
