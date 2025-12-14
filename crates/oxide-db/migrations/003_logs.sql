-- Step logs table (append-optimized)
CREATE TABLE IF NOT EXISTS step_logs (
    id BIGSERIAL PRIMARY KEY,
    step_id UUID NOT NULL REFERENCES steps(id) ON DELETE CASCADE,
    stream VARCHAR(10) NOT NULL CHECK (stream IN ('stdout', 'stderr')),
    line_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_step_logs_step_id ON step_logs(step_id);
CREATE INDEX idx_step_logs_step_line ON step_logs(step_id, line_number);
