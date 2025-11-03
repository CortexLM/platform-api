use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::models::{JobCache, JobStatus};
use crate::state::AppState;
use platform_api_models::ValidatorChallengeState;

/// Request to send a job to validators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributeJobRequest {
    pub job_id: String,
    pub job_name: String,
    pub payload: Value,
    pub compose_hash: String,
    pub challenge_id: String,
    pub challenge_cvm_ws_url: Option<String>, // URL to forward results back
}

/// Result from distributing a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributeJobResponse {
    pub job_id: String,
    pub distributed: bool,
    pub validator_count: usize,
    pub assigned_validators: Vec<String>,
}

/// Job result from validator to forward to challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: String,
    pub result: Value,
    pub error: Option<String>,
}

/// Job distributor manages distribution of jobs from challenge SDK to validators
pub struct JobDistributor {
    state: AppState,
}

impl JobDistributor {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Distribute a job to active validators for a specific compose_hash
    pub async fn distribute_job_to_validators(
        &self,
        request: DistributeJobRequest,
    ) -> Result<DistributeJobResponse> {
        info!(
            job_id = &request.job_id,
            compose_hash = &request.compose_hash,
            "Distributing job to validators"
        );

        // Get count of active validators
        let validator_count = self
            .state
            .get_validator_count(&request.compose_hash)
            .await;

        if validator_count == 0 {
            warn!(
                job_id = &request.job_id,
                compose_hash = &request.compose_hash,
                "No active validators available for job"
            );
            return Ok(DistributeJobResponse {
                job_id: request.job_id.clone(),
                distributed: false,
                validator_count: 0,
                assigned_validators: Vec::new(),
            });
        }

        // Find active validators for this compose_hash
        let active_validators = self
            .get_active_validators_for_compose_hash(&request.compose_hash)
            .await;

        if active_validators.is_empty() {
            warn!(
                job_id = &request.job_id,
                "No active validators found despite count > 0"
            );
            return Ok(DistributeJobResponse {
                job_id: request.job_id.clone(),
                distributed: false,
                validator_count,
                assigned_validators: Vec::new(),
            });
        }

        // Create job cache entry
        let mut job_cache = JobCache::new(
            request.job_id.clone(),
            request.challenge_id.clone(),
            request.compose_hash.clone(),
            request.challenge_cvm_ws_url.clone(),
        );
        job_cache.mark_distributing();

        // Store in job cache in AppState
        {
            let mut cache = self.state.job_cache.write().await;
            cache.insert(request.job_id.clone(), job_cache.clone());
        }

        // Prepare job message for validators
        let job_message = serde_json::json!({
            "type": "job_execute",
            "job_id": request.job_id.clone(),
            "job_name": request.job_name,
            "payload": request.payload,
            "challenge_id": request.challenge_id,
            "compose_hash": request.compose_hash,
        });

        let job_message_str = serde_json::to_string(&job_message)
            .context("Failed to serialize job message")?;

        // Send job to each active validator via WebSocket
        let mut assigned_validators = Vec::new();
        let validator_connections = self.state.validator_connections.read().await;

        for validator_hotkey in &active_validators {
            if let Some(conn) = validator_connections.get(validator_hotkey) {
                if let Some(sender) = &conn.message_sender {
                    // Send job message via WebSocket channel
                    if let Err(e) = sender.try_send(job_message_str.clone()) {
                        warn!(
                            validator_hotkey = validator_hotkey,
                            error = %e,
                            "Failed to send job to validator"
                        );
                        continue;
                    }
                    
                    job_cache.assigned_validators.push(validator_hotkey.clone());
                    assigned_validators.push(validator_hotkey.clone());
                    info!(
                        job_id = &request.job_id,
                        validator_hotkey = validator_hotkey,
                        "Sent job to validator"
                    );
                } else {
                    warn!(
                        validator_hotkey = validator_hotkey,
                        "Validator connection has no message sender"
                    );
                }
            } else {
                warn!(
                    validator_hotkey = validator_hotkey,
                    "Validator connection not found"
                );
            }
        }

        // Update job cache status
        if !assigned_validators.is_empty() {
            job_cache.mark_running(assigned_validators[0].clone());
            
            let mut cache = self.state.job_cache.write().await;
            cache.insert(request.job_id.clone(), job_cache);
        } else {
            // No validators assigned, mark as failed
            job_cache.mark_failed();
            let mut cache = self.state.job_cache.write().await;
            cache.insert(request.job_id.clone(), job_cache);
        }

        Ok(DistributeJobResponse {
            job_id: request.job_id,
            distributed: !assigned_validators.is_empty(),
            validator_count,
            assigned_validators,
        })
    }

    /// Get list of active validator hotkeys for a specific compose_hash
    async fn get_active_validators_for_compose_hash(
        &self,
        compose_hash: &str,
    ) -> Vec<String> {
        let status_map = self.state.validator_challenge_status.read().await;
        let mut validators = Vec::new();

        for (hotkey, challenge_statuses) in status_map.iter() {
            if let Some(status) = challenge_statuses.get(compose_hash) {
                if matches!(status.state, ValidatorChallengeState::Active) {
                    validators.push(hotkey.clone());
                }
            }
        }

        validators
    }

    /// Forward job result from validator to challenge CVM
    pub async fn forward_job_result(&self, result: JobResult) -> Result<()> {
        info!(
            job_id = &result.job_id,
            "Forwarding job result to challenge CVM"
        );

        // Find job cache entry
        let job_cache = {
            let cache = self.state.job_cache.read().await;
            cache.get(&result.job_id).cloned()
        };

        if let Some(mut job_cache) = job_cache {
            // Mark job as completed or failed
            if result.error.is_some() {
                job_cache.mark_failed();
            } else {
                job_cache.mark_completed();
            }
            
            // Update cache
            {
                let mut cache = self.state.job_cache.write().await;
                cache.insert(result.job_id.clone(), job_cache.clone());
            }
            
            // Forward result to challenge CVM if URL is available
            if let Some(challenge_cvm_ws_url) = &job_cache.challenge_cvm_ws_url {
                // TODO: Send result to challenge CVM via WebSocket
                // This will be handled in challenge_ws.rs when we receive job_result from validators
                info!(
                    job_id = &result.job_id,
                    challenge_cvm_url = challenge_cvm_ws_url,
                    "Job result will be forwarded to challenge CVM"
                );
                // The actual forwarding will happen in websocket.rs handler for job_result
            } else {
                warn!(
                    job_id = &result.job_id,
                    "No challenge_cvm_ws_url in job cache, cannot forward result"
                );
            }
            
            Ok(())
        } else {
            Err(anyhow!("Job {} not found in cache", result.job_id))
        }
    }
}

