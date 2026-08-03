-- Approval gates
--
-- Columns exist for the fields something actually filters or sorts on; the
-- rest of the gate — approvers, allowed approvers, policy flags — lives in the
-- gate JSONB, the same way pipelines store their definition.
CREATE TABLE IF NOT EXISTS approval_gates (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    stage_name VARCHAR(255) NOT NULL,
    environment VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    required_approvers INTEGER NOT NULL DEFAULT 1,
    current_approvals INTEGER NOT NULL DEFAULT 0,
    gate JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_approval_gates_run ON approval_gates(run_id);
CREATE INDEX idx_approval_gates_status ON approval_gates(status);
-- Expiry sweeps only ever care about gates still waiting.
CREATE INDEX idx_approval_gates_pending_expiry ON approval_gates(expires_at)
    WHERE status = 'pending';
