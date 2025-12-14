-- Secrets table (encrypted at rest)
CREATE TABLE IF NOT EXISTS secrets (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    scope VARCHAR(50) NOT NULL,
    scope_id VARCHAR(255),
    provider VARCHAR(50) NOT NULL DEFAULT 'internal',
    encrypted_value BYTEA,
    provider_config JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    created_by VARCHAR(255),
    last_used_at TIMESTAMPTZ,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX idx_secrets_name_scope ON secrets(name, scope, COALESCE(scope_id, ''));
CREATE INDEX idx_secrets_scope ON secrets(scope, scope_id);

-- Cache metadata table
CREATE TABLE IF NOT EXISTS cache_entries (
    id UUID PRIMARY KEY,
    key VARCHAR(512) NOT NULL,
    version VARCHAR(255),
    size_bytes BIGINT NOT NULL,
    compression VARCHAR(20) NOT NULL DEFAULT 'none',
    checksum_sha256 VARCHAR(64) NOT NULL,
    scope VARCHAR(50) NOT NULL,
    scope_id VARCHAR(255),
    storage_path TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    access_count INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX idx_cache_key_scope ON cache_entries(key, scope, COALESCE(scope_id, ''));
CREATE INDEX idx_cache_scope ON cache_entries(scope, scope_id);
CREATE INDEX idx_cache_expires ON cache_entries(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_cache_lru ON cache_entries(last_accessed_at);

-- Audit log table
CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,
    actor VARCHAR(255),
    changes JSONB,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_actor ON audit_log(actor) WHERE actor IS NOT NULL;
