-- Migration: Add Redis logging support and job progress columns
-- Created: 2025-01-26

-- Add progress tracking columns to jobs table
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS progress_percent DECIMAL(5,2);
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS total_tasks INTEGER;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS completed_tasks INTEGER;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS resolved_tasks INTEGER;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS unresolved_tasks INTEGER;

-- Create job_test_results table for detailed test outcomes
CREATE TABLE IF NOT EXISTS job_test_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    challenge_id UUID NOT NULL,
    task_id VARCHAR(255) NOT NULL,
    test_name VARCHAR(255),
    status VARCHAR(50) NOT NULL, -- 'passed', 'failed', 'error', 'skipped'
    is_resolved BOOLEAN DEFAULT false,
    error_message TEXT,
    execution_time_ms BIGINT,
    output_text TEXT,
    logs JSONB,
    metrics JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_job_test_results_job_id ON job_test_results(job_id);
CREATE INDEX idx_job_test_results_challenge_id ON job_test_results(challenge_id);
CREATE INDEX idx_job_test_results_task_id ON job_test_results(task_id);
CREATE INDEX idx_job_test_results_status ON job_test_results(status);
CREATE INDEX idx_job_test_results_created_at ON job_test_results(created_at);

