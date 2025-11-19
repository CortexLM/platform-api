-- Migration: Add os_image_hash and hardware spec columns to vm_compose_configs table
-- Created: 2025-11-19
-- Purpose: Store OS image hash and hardware specifications for VM configurations

-- Add os_image_hash column (hex-encoded hash of the OS image)
ALTER TABLE vm_compose_configs
ADD COLUMN IF NOT EXISTS os_image_hash VARCHAR(128);

-- Add hardware specification columns
ALTER TABLE vm_compose_configs
ADD COLUMN IF NOT EXISTS vcpu INTEGER DEFAULT 16;

ALTER TABLE vm_compose_configs
ADD COLUMN IF NOT EXISTS memory_mb INTEGER DEFAULT 16384;

ALTER TABLE vm_compose_configs
ADD COLUMN IF NOT EXISTS disk_gb INTEGER DEFAULT 200;

ALTER TABLE vm_compose_configs
ADD COLUMN IF NOT EXISTS image_version VARCHAR(64) DEFAULT 'dstack-0.5.2';

-- Create index on os_image_hash for faster lookups
CREATE INDEX IF NOT EXISTS idx_vm_compose_configs_os_image_hash 
ON vm_compose_configs(os_image_hash);

-- Update validator_vm with default values
-- TODO: Replace with actual os_image_hash from dstack registry
UPDATE vm_compose_configs
SET 
    os_image_hash = '',  -- Empty for now - will be filled from validator's vm_config
    vcpu = 16,
    memory_mb = 16384,
    disk_gb = 200,
    image_version = 'dstack-0.5.2'
WHERE vm_type = 'validator_vm';

-- Add comment explaining os_image_hash
COMMENT ON COLUMN vm_compose_configs.os_image_hash IS 
'Hex-encoded SHA256 hash of the OS image (from dstack registry). Empty string means extract from validator vm_config.';

