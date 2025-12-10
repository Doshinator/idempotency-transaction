-- Add migration script here
CREATE TABLE idempotency_keys (
    idempotency_key TEXT PRIMARY KEY,
    user_session_id UUID NOT NULL,
    request_hash TEXT NOT NULL,
    response_body JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);