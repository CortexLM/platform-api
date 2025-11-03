-- Migration: Create challenge orchestration tables
-- Created: 2025-01-26

-- Table: validator_challenge_status
-- Tracks the status of each challenge served by validators
CREATE TABLE IF NOT EXISTS validator_challenge_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    validator_hotkey VARCHAR(255) NOT NULL,
    compose_hash VARCHAR(64) NOT NULL,
    state VARCHAR(50) NOT NULL,
    last_heartbeat TIMESTAMP WITH TIME ZONE NOT NULL,
    penalty_reason TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(validator_hotkey, compose_hash)
);

CREATE INDEX idx_validator_challenge_status_validator ON validator_challenge_status(validator_hotkey);
CREATE INDEX idx_validator_challenge_status_compose ON validator_challenge_status(compose_hash);
CREATE INDEX idx_validator_challenge_status_state ON validator_challenge_status(state);

-- Table: challenge_results
-- Stores results from challenge job executions
CREATE TABLE IF NOT EXISTS challenge_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id UUID NOT NULL,
    validator_hotkey VARCHAR(255) NOT NULL,
    compose_hash VARCHAR(64) NOT NULL,
    artifact_id VARCHAR(255) NOT NULL,
    score DECIMAL(10, 6) NOT NULL,
    weight DECIMAL(10, 6) NOT NULL,
    justification TEXT NOT NULL,
    execution_time_ms BIGINT,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_challenge_results_run_id ON challenge_results(run_id);
CREATE INDEX idx_challenge_results_validator ON challenge_results(validator_hotkey);
CREATE INDEX idx_challenge_results_compose ON challenge_results(compose_hash);
CREATE INDEX idx_challenge_results_created_at ON challenge_results(created_at);

-- Table: emissions
-- Tracks emissions per challenge and validator
CREATE TABLE IF NOT EXISTS emissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    block_height BIGINT NOT NULL,
    compose_hash VARCHAR(64) NOT NULL,
    emission_share DECIMAL(10, 6) NOT NULL,
    owner_hotkey VARCHAR(255) NOT NULL,
    validator_hotkey VARCHAR(255),
    amount DECIMAL(20, 10) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_emissions_block_height ON emissions(block_height);
CREATE INDEX idx_emissions_compose_hash ON emissions(compose_hash);
CREATE INDEX idx_emissions_owner ON emissions(owner_hotkey);
CREATE INDEX idx_emissions_validator ON emissions(validator_hotkey);
CREATE INDEX idx_emissions_created_at ON emissions(created_at);

-- Table: scoring_runs
-- Tracks scoring runs triggered by block height
CREATE TABLE IF NOT EXISTS scoring_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    block_height BIGINT NOT NULL,
    triggered_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(50) NOT NULL,
    results_count INTEGER DEFAULT 0,
    error_message TEXT
);

CREATE INDEX idx_scoring_runs_block_height ON scoring_runs(block_height);
CREATE INDEX idx_scoring_runs_status ON scoring_runs(status);

