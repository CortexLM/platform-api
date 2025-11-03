-- Migration: Add mechanism_id, weight, description, mermaid_chart, github_repo to challenges
-- Created: 2025-01-26

-- Create challenges table if it doesn't exist
CREATE TABLE IF NOT EXISTS challenges (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    compose_hash VARCHAR NOT NULL UNIQUE,
    compose_yaml TEXT NOT NULL,
    version VARCHAR NOT NULL,
    images TEXT[] NOT NULL,
    resources JSONB NOT NULL,
    ports JSONB NOT NULL,
    env JSONB NOT NULL,
    emission_share DOUBLE PRECISION NOT NULL,
    mechanism_id VARCHAR NOT NULL,
    weight DOUBLE PRECISION,
    description TEXT,
    mermaid_chart TEXT,
    github_repo VARCHAR,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

-- Add mechanism_id column if it doesn't exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'mechanism_id') THEN
        ALTER TABLE challenges ADD COLUMN mechanism_id VARCHAR NOT NULL DEFAULT 'default';
    END IF;
END $$;

-- Add weight column if it doesn't exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'weight') THEN
        ALTER TABLE challenges ADD COLUMN weight DOUBLE PRECISION;
    END IF;
END $$;

-- Add description column if it doesn't exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'description') THEN
        ALTER TABLE challenges ADD COLUMN description TEXT;
    END IF;
END $$;

-- Add mermaid_chart column if it doesn't exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'mermaid_chart') THEN
        ALTER TABLE challenges ADD COLUMN mermaid_chart TEXT;
    END IF;
END $$;

-- Add github_repo column if it doesn't exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'github_repo') THEN
        ALTER TABLE challenges ADD COLUMN github_repo VARCHAR;
    END IF;
END $$;

-- Create index on mechanism_id
CREATE INDEX IF NOT EXISTS idx_challenges_mechanism_id ON challenges(mechanism_id);

-- Create index on mechanism_id and compose_hash for quick lookups
CREATE INDEX IF NOT EXISTS idx_challenges_mechanism_compose ON challenges(mechanism_id, compose_hash);

