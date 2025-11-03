-- Migration: Add dstack_image field to challenges
-- Created: 2025-10-27

-- Add dstack_image column if it doesn't exist
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'challenges' AND column_name = 'dstack_image') THEN
        ALTER TABLE challenges ADD COLUMN dstack_image VARCHAR DEFAULT 'dstack-0.5.2';
    END IF;
END $$;





