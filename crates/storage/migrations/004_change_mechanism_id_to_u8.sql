-- Migration: Change mechanism_id from VARCHAR to SMALLINT (u8)
-- Created: 2025-01-26

-- First, convert existing string values to integers
-- Try to parse mechanism_id strings to integers, default to 0 if parsing fails
DO $$
DECLARE
    row_record RECORD;
    parsed_id SMALLINT;
BEGIN
    -- Add temporary column for numeric mechanism_id
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'mechanism_id_temp') THEN
        ALTER TABLE challenges ADD COLUMN mechanism_id_temp SMALLINT;
    END IF;
    
    -- Convert existing VARCHAR values to SMALLINT
    FOR row_record IN SELECT id, mechanism_id FROM challenges LOOP
        BEGIN
            -- Try to parse as integer, default to 0 if it fails
            parsed_id := COALESCE(row_record.mechanism_id::SMALLINT, 0);
        EXCEPTION WHEN OTHERS THEN
            -- If parsing fails, use 0 as default
            parsed_id := 0;
        END;
        
        UPDATE challenges 
        SET mechanism_id_temp = parsed_id 
        WHERE id = row_record.id;
    END LOOP;
    
    -- Drop old VARCHAR column
    ALTER TABLE challenges DROP COLUMN IF EXISTS mechanism_id;
    
    -- Rename temp column to mechanism_id
    ALTER TABLE challenges RENAME COLUMN mechanism_id_temp TO mechanism_id;
    
    -- Set NOT NULL constraint
    ALTER TABLE challenges ALTER COLUMN mechanism_id SET NOT NULL;
    ALTER TABLE challenges ALTER COLUMN mechanism_id SET DEFAULT 0;
END $$;

-- Recreate index on mechanism_id (now SMALLINT)
DROP INDEX IF EXISTS idx_challenges_mechanism_id;
CREATE INDEX IF NOT EXISTS idx_challenges_mechanism_id ON challenges(mechanism_id);

-- Recreate composite index
DROP INDEX IF EXISTS idx_challenges_mechanism_compose;
CREATE INDEX IF NOT EXISTS idx_challenges_mechanism_compose ON challenges(mechanism_id, compose_hash);

