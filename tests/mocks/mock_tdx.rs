// Mock TDX attestation client for testing
// TDX requires hardware, so we always mock it

use anyhow::Result;
use platform_api_models::{AttestationRequest, AttestationResponse, AttestationStatus};
use chrono::Utc;

/// Mock TDX verifier for testing
pub struct MockTdxVerifier {
    pub should_succeed: bool,
    pub mock_compose_hash: Option<String>,
}

impl MockTdxVerifier {
    pub fn new() -> Self {
        Self {
            should_succeed: true,
            mock_compose_hash: None,
        }
    }

    pub fn with_success(mut self, succeed: bool) -> Self {
        self.should_succeed = succeed;
        self
    }

    pub fn with_compose_hash(mut self, hash: String) -> Self {
        self.mock_compose_hash = Some(hash);
        self
    }

    /// Mock verification of TDX attestation request
    pub async fn verify_attestation(&self, request: &AttestationRequest) -> Result<AttestationResponse> {
        if !self.should_succeed {
            return Ok(AttestationResponse {
                session_token: String::new(),
                status: AttestationStatus::Failed,
                expires_at: Utc::now(),
                verified_measurements: vec![],
                policy: String::new(),
                error: Some("Mock TDX verification failed".to_string()),
            });
        }

        // Validate structure even in mock
        if request.quote.is_none() {
            return Ok(AttestationResponse {
                session_token: String::new(),
                status: AttestationStatus::Failed,
                expires_at: Utc::now(),
                verified_measurements: vec![],
                policy: String::new(),
                error: Some("Missing quote in attestation request".to_string()),
            });
        }

        // Return successful mock response
        Ok(AttestationResponse {
            session_token: "mock-session-token".to_string(),
            status: AttestationStatus::Verified,
            expires_at: Utc::now() + chrono::Duration::hours(1),
            verified_measurements: request.measurements.clone(),
            policy: "mock-policy".to_string(),
            error: None,
        })
    }

    /// Mock getting compose hash from TDX attestation
    pub async fn get_compose_hash(&self) -> Result<String> {
        if let Some(hash) = &self.mock_compose_hash {
            Ok(hash.clone())
        } else {
            Ok("mock-compose-hash-12345".to_string())
        }
    }
}

impl Default for MockTdxVerifier {
    fn default() -> Self {
        Self::new()
    }
}

