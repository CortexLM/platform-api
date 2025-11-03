use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use serde::Serialize;
use jsonwebtoken::{encode, Header, EncodingKey, DecodingKey, Algorithm};
use platform_api_models::{AttestationRequest, AttestationResponse, AttestationSession, AttestationPolicy};

mod verifier;
pub use verifier::*;

mod config;
pub use config::*;

/// Attestation service for TDX VM verification
pub struct AttestationService {
    config: AttestationConfig,
    verifier: TdxVerifier,
    sessions: Arc<tokio::sync::RwLock<HashMap<Uuid, AttestationSession>>>,
    nonces: Arc<tokio::sync::RwLock<HashMap<String, NonceInfo>>>,
    signing_key: EncodingKey,
    decoding_key: DecodingKey,
}

/// Nonce information
#[derive(Debug, Clone)]
struct NonceInfo {
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl AttestationService {
    pub fn new(config: &AttestationConfig) -> Result<Self> {
        // Security check: prevent use of default JWT secret in production
        const DEFAULT_SECRET: &str = "change-me-in-production";
        if config.jwt_secret == DEFAULT_SECRET {
            return Err(anyhow::anyhow!(
                "Security error: Default JWT secret '{}' cannot be used. Please set JWT_SECRET environment variable with a strong secret.",
                DEFAULT_SECRET
            ));
        }
        
        let signing_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
        
        let verifier = TdxVerifier::new(config.clone());
        
        Ok(Self {
            config: config.clone(),
            verifier,
            sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            nonces: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            signing_key,
            decoding_key,
        })
    }

    pub async fn verify_attestation(&self, request: AttestationRequest) -> Result<AttestationResponse> {
        self.verify_attestation_with_event_log(request, None).await
    }
    
    pub async fn verify_attestation_with_event_log(&self, request: AttestationRequest, event_log: Option<&str>) -> Result<AttestationResponse> {
        tracing::info!("Verifying attestation request");
        
        // Verify the attestation with the verifier
        let verification_result = TdxVerifier::verify_static(&self.verifier, &request, event_log).await?;
        
        if !verification_result.is_valid {
            return Ok(AttestationResponse {
                session_token: String::new(),
                status: platform_api_models::AttestationStatus::Failed,
                expires_at: Utc::now(),
                verified_measurements: vec![],
                policy: String::new(),
                error: Some(verification_result.error.unwrap_or_else(|| "Verification failed".to_string())),
            });
        }

        // Generate session token
        let session_id = Uuid::new_v4();
        let session_token = self.generate_grant_token(&session_id, &verification_result)?;
        let expires_at = Utc::now() + Duration::seconds(self.config.session_timeout as i64);

        // Store session
        // Derive validator_hotkey from verified TEE identity (app_id and instance_id)
        let validator_hotkey = {
            let app_id_str = verification_result.app_id.as_ref()
                .map(|v| hex::encode(v))
                .unwrap_or_else(|| "unknown".to_string());
            let instance_id_str = verification_result.instance_id.as_ref()
                .map(|v| hex::encode(v))
                .unwrap_or_else(|| "unknown".to_string());
            format!("validator-{}-{}", app_id_str, instance_id_str)
        };
        
        let session = AttestationSession {
            id: session_id,
            session_token: session_token.clone(),
            attestation_type: request.attestation_type,
            status: platform_api_models::AttestationStatus::Verified,
            validator_hotkey,
            created_at: Utc::now(),
            expires_at,
            verified_measurements: verification_result.measurements.clone(),
            policy: String::new(),
            key_releases: vec![],
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);

        Ok(AttestationResponse {
            session_token,
            status: platform_api_models::AttestationStatus::Verified,
            expires_at,
            verified_measurements: verification_result.measurements,
            policy: String::new(),
            error: None,
        })
    }

    pub async fn get_session(&self, id: Uuid) -> Result<AttestationSession> {
        let sessions = self.sessions.read().await;
        sessions.get(&id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found"))
    }

    pub async fn list_policies(&self) -> Result<Vec<AttestationPolicy>> {
        Ok(vec![])
    }

    pub async fn get_policy(&self, _id: &str) -> Result<AttestationPolicy> {
        Err(anyhow::anyhow!("Policy not found"))
    }

    pub fn verify_token(&self, token: &str) -> Result<serde_json::Value> {
        use jsonwebtoken::{decode, Validation, Algorithm};
        
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["platform-executor"]);
        
        let claims = decode::<serde_json::Value>(token, &self.decoding_key, &validation)
            .context("Failed to decode JWT token")?;
        
        // Verify expiration
        let exp = claims.claims.get("exp")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Missing exp claim"))?;
        
        let now = chrono::Utc::now().timestamp() as u64;
        if exp < now {
            return Err(anyhow::anyhow!("Token expired"));
        }
        
        // Verify required claims
        if claims.claims.get("app_id").is_none() {
            return Err(anyhow::anyhow!("Missing app_id claim"));
        }
        
        if claims.claims.get("instance_id").is_none() {
            return Err(anyhow::anyhow!("Missing instance_id claim"));
        }
        
        Ok(claims.claims)
    }

    fn generate_grant_token(&self, session_id: &Uuid, verification: &VerificationResult) -> Result<String> {
        let claims = GrantClaims {
            sub: session_id.to_string(),
            jti: session_id.to_string(),
            aud: "platform-executor".to_string(),
            exp: (Utc::now() + Duration::seconds(300)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
            app_id: hex::encode(&verification.app_id.clone().unwrap_or_default()),
            instance_id: hex::encode(&verification.instance_id.clone().unwrap_or_default()),
            device_id: hex::encode(&verification.device_id.clone().unwrap_or_default()),
        };

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.signing_key)?;
        Ok(token)
    }
}

/// Grant JWT claims
#[derive(Debug, Serialize)]
struct GrantClaims {
    sub: String,
    jti: String,
    aud: String,
    exp: usize,
    iat: usize,
    app_id: String,
    instance_id: String,
    device_id: String,
}

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub measurements: Vec<Vec<u8>>,
    pub app_id: Option<Vec<u8>>,
    pub instance_id: Option<Vec<u8>>,
    pub device_id: Option<Vec<u8>>,
    pub error: Option<String>,
}

