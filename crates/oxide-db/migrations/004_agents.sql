-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    labels TEXT[] NOT NULL DEFAULT '{}',
    version VARCHAR(50),
    os VARCHAR(50) NOT NULL,
    arch VARCHAR(50) NOT NULL,
    capabilities JSONB NOT NULL DEFAULT '[]',
    max_concurrent_jobs INTEGER NOT NULL DEFAULT 1,
    status VARCHAR(50) NOT NULL DEFAULT 'offline',
    current_run_id UUID REFERENCES runs(id) ON DELETE SET NULL,
    system_metrics JSONB,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_agents_name ON agents(name);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_available ON agents(status, max_concurrent_jobs) WHERE status = 'idle';
CREATE INDEX idx_agents_labels ON agents USING GIN(labels);
