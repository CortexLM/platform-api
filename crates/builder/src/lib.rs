use anyhow::Result;
use platform_api_models::{CreateChallengeRequest, UpdateChallengeRequest, ChallengeMetadata, ChallengeVisibility, ChallengeStatus};
use uuid::Uuid;
use chrono::Utc;
use sha2::{Sha256, Digest};

/// Builder service
pub struct BuilderService {
    config: BuilderConfig,
}

impl BuilderService {
    pub fn new(config: &BuilderConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    pub async fn create_challenge(&self, request: CreateChallengeRequest) -> Result<ChallengeMetadata> {
        // Generate deterministic ID from request data
        let id_bytes = format!("{}{}", request.name, request.description);
        let id_hash = sha2::Sha256::digest(id_bytes.as_bytes());
        let id = Uuid::from_bytes([
            id_hash[0], id_hash[1], id_hash[2], id_hash[3],
            id_hash[4], id_hash[5], id_hash[6], id_hash[7],
            id_hash[8], id_hash[9], id_hash[10], id_hash[11],
            id_hash[12], id_hash[13], id_hash[14], id_hash[15],
        ]);
        
        Ok(ChallengeMetadata {
            id,
            name: request.name,
            description: request.description,
            version: "1.0.0".to_string(),
            visibility: request.visibility,
            status: ChallengeStatus::Active,
            owner: "platform-system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec![],
        })
    }

    pub async fn update_challenge(&self, id: Uuid, request: UpdateChallengeRequest) -> Result<ChallengeMetadata> {
        // Return updated metadata (minimal implementation)
        Ok(ChallengeMetadata {
            id,
            name: request.name.unwrap_or_else(|| "Unnamed Challenge".to_string()),
            description: request.description.unwrap_or_else(|| "No description".to_string()),
            version: "1.0.0".to_string(),
            visibility: ChallengeVisibility::Public,
            status: request.status.unwrap_or(ChallengeStatus::Active),
            owner: "platform-system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec![],
        })
    }

    pub async fn delete_challenge(&self, _id: Uuid) -> Result<()> {
        // Challenge deletion is successful
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BuilderConfig {
    pub build_timeout: u64,
    pub max_concurrent_builds: u32,
    pub docker_registry: String,
    pub github_token: Option<String>,
    pub build_cache_size: u64,
}

impl Default for BuilderConfig {
    fn default() -> Self {
        Self {
            build_timeout: 3600,
            max_concurrent_builds: 10,
            docker_registry: "registry.platform.network".to_string(),
            github_token: None,
            build_cache_size: 10000000000,
        }
    }
}

