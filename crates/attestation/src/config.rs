use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationConfig {
    pub dcap_enabled: bool,
    pub sev_enabled: bool,
    pub tdx_enabled: bool,
    pub policy_store_path: String,
    pub verification_timeout: u64,
    pub session_timeout: u64,
    pub jwt_secret: String,
    pub verifier_url: Option<String>,
}

impl Default for AttestationConfig {
    fn default() -> Self {
        Self {
            dcap_enabled: false,
            sev_enabled: false,
            tdx_enabled: true,
            policy_store_path: "/var/lib/platform-api/policies".to_string(),
            verification_timeout: 30,
            session_timeout: 300,
            jwt_secret: "change-me-in-production".to_string(),
            verifier_url: None,
        }
    }
}

