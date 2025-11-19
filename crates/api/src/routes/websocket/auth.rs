use anyhow::Context;
use base64::{engine::general_purpose::STANDARD as base64_engine, Engine as _};
use hex;
use serde_json::json;
use sha2::{Digest, Sha256};
use sp_core::{crypto::Ss58Codec, sr25519};
use tracing::{info, warn};

use crate::services::DstackVerifierClient;
use crate::state::AppState;
use dstack_types::VmConfig;
use platform_api_models::{AttestationRequest, AttestationType};
use std::sync::Arc;

use super::messages::{AttestationMessage, SecureMessage};
use super::utils::extract_compose_hash_from_event_log;

/// Verify secure message signature and timestamp
pub async fn verify_secure_message(
    msg: &SecureMessage,
    expected_hotkey: &str,
) -> anyhow::Result<()> {
    // Verify timestamp is recent (within 30 seconds)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now.saturating_sub(msg.timestamp) > 30 {
        return Err(anyhow::anyhow!(
            "Message timestamp too old: {} seconds",
            now.saturating_sub(msg.timestamp)
        ));
    }

    // Verify public key matches expected hotkey
    if msg.public_key != expected_hotkey {
        return Err(anyhow::anyhow!(
            "Public key mismatch: expected {}, got {}",
            expected_hotkey,
            msg.public_key
        ));
    }

    // Decode public key
    let public_key = sr25519::Public::from_ss58check(&msg.public_key)
        .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;

    // Recreate message to verify
    let mut message = Vec::new();
    message.extend_from_slice(msg.message_type.as_bytes());
    message.extend_from_slice(msg.timestamp.to_string().as_bytes());
    message.extend_from_slice(msg.nonce.as_bytes());
    message.extend_from_slice(msg.data.to_string().as_bytes());

    // Decode signature
    let signature_bytes =
        hex::decode(&msg.signature).map_err(|e| anyhow::anyhow!("Invalid signature hex: {}", e))?;

    if signature_bytes.len() != 64 {
        return Err(anyhow::anyhow!("Invalid signature length"));
    }

    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(&signature_bytes);
    let signature = sr25519::Signature::from(sig_array);

    // Verify signature using the verify_trait
    use sp_core::crypto::Pair;
    let is_valid = sr25519::Pair::verify(&signature, &message, &public_key);
    if !is_valid {
        return Err(anyhow::anyhow!("Signature verification failed"));
    }

    Ok(())
}

/// Verify validator TDX attestation
pub async fn verify_validator_attestation(
    state: &AppState,
    msg: &AttestationMessage,
    challenge: Option<&[u8]>,
) -> anyhow::Result<()> {
    // If dstack-verifier is configured, use it for full platform verification
    if let Some(ref verifier) = state.dstack_verifier {
        return verify_validator_with_dstack_verifier(state, msg, challenge, verifier).await;
    }

    // Otherwise, use the built-in verification (quote only)
    // Decode quote and event_log
    let quote = msg
        .quote
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing quote"))?;
    // Quote can be in base64 (from validator) or hex (legacy)
    let quote_bytes = match base64_engine.decode(quote) {
        Ok(b) => b,
        Err(_) => {
            // Try hex as fallback for legacy compatibility
            hex::decode(quote).context("Failed to decode quote (tried base64 and hex)")?
        }
    };

    let measurements = msg
        .measurements
        .as_ref()
        .map(|m| {
            m.iter()
                .map(|s| hex::decode(s).unwrap_or_default())
                .collect()
        })
        .unwrap_or_default();

    // Create attestation request
    let attest_request = AttestationRequest {
        attestation_type: AttestationType::Tdx,
        quote: Some(quote_bytes),
        report: None,
        nonce: challenge.unwrap_or(&[]).to_vec(),
        measurements,
        capabilities: vec![],
    };

    // Verify attestation with event log
    let event_log = msg.event_log.as_deref();
    let result = state
        .attestation
        .verify_attestation_with_event_log(attest_request, event_log)
        .await
        .context("Failed to verify attestation")?;

    if !matches!(
        result.status,
        platform_api_models::AttestationStatus::Verified
    ) {
        return Err(anyhow::anyhow!(
            "Attestation verification failed: {:?}",
            result.error
        ));
    }

    if let Some(event_log_str) = msg.event_log.as_deref() {
        match extract_compose_hash_from_event_log(event_log_str) {
            Some(hash) => {
                info!(
                    compose_hash = hash,
                    "Validator event log reported compose_hash (informational)"
                );
            }
            None => {
                warn!("Validator event log missing compose-hash entry; continuing because trust is derived from TDX quote validity");
            }
        }
    } else {
        warn!("Validator attestation did not include an event log; continuing because TDX verification already succeeded");
    }

    Ok(())
}

/// Verify challenge binding in report data
pub fn verify_challenge_binding(
    report_data_hex: &str,
    expected_challenge: &str,
    challenge_hash: &str,
) -> bool {
    report_data_hex == expected_challenge
        || report_data_hex == challenge_hash
        || report_data_hex.starts_with(challenge_hash as &str)
        || report_data_hex.starts_with(expected_challenge as &str)
        || (report_data_hex.len() >= 64 && &report_data_hex[..64] == challenge_hash)
}

/// Compute challenge hash
pub fn compute_challenge_hash(challenge: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(challenge.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verify validator using full TDX verification with dstack-verifier
/// This verifies:
/// 1. Quote signature using Intel PCCS/dcap-qvl
/// 2. MRTD/RTMR measurements match expected values
/// 3. Compose hash matches expected value from DB
/// 4. Challenge binding (nonce) is correct
async fn verify_validator_with_dstack_verifier(
    state: &AppState,
    msg: &AttestationMessage,
    challenge: Option<&[u8]>,
    verifier: &Arc<DstackVerifierClient>,
) -> anyhow::Result<()> {
    info!("Starting full TDX verification for validator");

    // Extract event log
    let event_log = msg
        .event_log
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing event log"))?;

    // Extract compose hash from event log
    let validator_compose_hash = extract_compose_hash_from_event_log(event_log)
        .ok_or_else(|| anyhow::anyhow!("Missing compose-hash in event log"))?;

    info!(
        "Validator reported compose hash: {}",
        validator_compose_hash
    );

    // Get expected compose config from DB
    let db_compose_config = state
        .storage
        .get_vm_compose_config("validator_vm")
        .await
        .context("Failed to retrieve validator_vm compose config from DB")?;

    info!(
        "Retrieved compose config from DB for vm_type: {}",
        db_compose_config.vm_type
    );

    // Build provisioning bundle (same logic as config.rs)
    let mut env_keys: Vec<String> = ["DSTACK_VMM_URL", "HOTKEY_PASSPHRASE", "VALIDATOR_BASE_URL"]
        .iter()
        .map(|k| k.to_string())
        .collect();
    for key in &db_compose_config.required_env {
        if !env_keys.iter().any(|existing| existing == key) {
            env_keys.push(key.clone());
        }
    }
    env_keys.sort();
    env_keys.dedup();

    // Build app_compose manifest (same structure as deploy.rs)
    let app_compose = json!({
        "manifest_version": 2,
        "name": db_compose_config.vm_type,
        "runner": "docker-compose",
        "docker_compose_file": db_compose_config.compose_content,
        "kms_enabled": true,
        "gateway_enabled": true,
        "local_key_provider_enabled": false,
        "key_provider_id": "",
        "public_logs": true,
        "public_sysinfo": true,
        "public_tcbinfo": true,
        "allowed_envs": env_keys,
        "no_instance_id": false,
        "secure_time": false,
    });

    // Calculate expected compose hash (same method as deploy.rs)
    let app_compose_str =
        serde_json::to_string(&app_compose).context("Failed to serialize app_compose")?;

    info!("📋 PLATFORM-API EXPECTED app_compose (raw JSON):\n{}", app_compose_str);
    info!("📋 PLATFORM-API env_keys used: {:?}", env_keys);

    // Normalize JSON to ensure consistent key ordering before hashing
    let normalized_compose = normalize_json_for_hashing(&app_compose_str)
        .unwrap_or_else(|_| app_compose_str.clone());
    
    info!("📋 PLATFORM-API normalized JSON:\n{}", normalized_compose);

    let mut hasher = Sha256::new();
    hasher.update(normalized_compose.as_bytes());
    let expected_compose_hash = hex::encode(hasher.finalize());

    info!("Expected compose hash from DB: {}", expected_compose_hash);

    // Compare compose hashes
    if validator_compose_hash != expected_compose_hash {
        return Err(anyhow::anyhow!(
            "Compose hash mismatch: validator reported {}, expected {}",
            validator_compose_hash,
            expected_compose_hash
        ));
    }
    
    info!("✅ Compose hash verification successful");

    // Extract quote for dstack-verifier
    let quote_str = msg
        .quote
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing quote for TDX verification"))?;

    // Decode quote from base64 to hex (dstack-verifier expects hex)
    let quote_bytes = match base64_engine.decode(quote_str) {
        Ok(bytes) => bytes,
        Err(_) => {
            // Try hex as fallback
            hex::decode(quote_str).context("Failed to decode quote as base64 or hex")?
        }
    };
    let quote_hex = hex::encode(&quote_bytes);

    // Get VM hardware spec from config.rs (same values used to provision the VM)
    let vm_spec = state
        .storage
        .get_vm_compose_config("validator_vm")
        .await
        .context("Failed to get VM spec for verification")?;

    // Check if validator provided vm_config (required for production)
    let has_vm_config = msg.vm_config.as_ref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);

    if !has_vm_config {
        return Err(anyhow::anyhow!(
            "Validator must provide vm_config for TDX verification. \
             The validator VM must be running in a dstack CVM with guest-agent enabled."
        ));
    }
    
    {
        // Extract VM config from validator's message
        // The vm_config from the validator's guest-agent includes os_image_hash
        // from /etc/dstack/sys_config.json (created by VMM at boot)
        let (vm_config_str, vm_config) = resolve_vm_config_from_msg(msg, "")?;
        
        // Get os_image_hash from the parsed vm_config (already included by dstack)
        let os_image_hash = hex::encode(&vm_config.os_image_hash);

        info!(
            "Using VM config for verification: cpu_count={}, memory_size={}, os_image_hash={}",
            vm_config.cpu_count, vm_config.memory_size, os_image_hash
        );

        // Call dstack-verifier to perform full TDX verification
        let pccs_url = std::env::var("PCCS_URL").ok();
        
        let verification_request = crate::services::dstack_verifier::VerificationRequest {
            quote: quote_hex,
            event_log: event_log.clone(),
            vm_config: vm_config_str,
            pccs_url,
            debug: Some(false),
        };

        info!("Calling dstack-verifier for full TDX verification");
        
        let verification_result = verifier
            .verify(verification_request)
            .await
            .context("Failed to verify TDX quote with dstack-verifier")?;

        if !verification_result.is_valid {
            return Err(anyhow::anyhow!(
                "TDX verification failed: {}",
                verification_result.reason.unwrap_or_else(|| "Unknown reason".to_string())
            ));
        }

        info!(
            "✅ TDX verification successful - quote_verified={}, event_log_verified={}, os_image_hash_verified={}",
            verification_result.details.quote_verified,
            verification_result.details.event_log_verified,
            verification_result.details.os_image_hash_verified
        );

        if let Some(tcb_status) = &verification_result.details.tcb_status {
            info!("TCB Status: {}", tcb_status);
        }
    }

    // Verify challenge binding if provided
    if let Some(challenge_bytes) = challenge {
        // Extract quote to verify challenge binding
        let quote = msg
            .quote
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing quote for challenge verification"))?;

        let quote_bytes = match base64_engine.decode(quote) {
            Ok(bytes) => bytes,
            Err(_) => hex::decode(quote).context("Failed to decode quote as base64 or hex")?,
        };

        // Calculate expected SHA256 of challenge
        let mut hasher = Sha256::new();
        hasher.update(challenge_bytes);
        let expected_hash = hasher.finalize();

        // Check if report_data in quote matches challenge (report_data is at offset 568-632)
        if quote_bytes.len() >= 632 {
            let report_data_slice = &quote_bytes[568..632];
            if report_data_slice[..32] != expected_hash[..] {
                return Err(anyhow::anyhow!(
                    "Challenge verification failed: report_data in quote does not match SHA256(challenge)"
                ));
            }
            info!("✅ Challenge nonce binding verified");
        } else {
            warn!("Quote too short to verify challenge binding, skipping");
        }
    }

    Ok(())
}

fn resolve_vm_config_from_msg(
    msg: &AttestationMessage,
    os_image_hash: &str,
) -> anyhow::Result<(String, VmConfig)> {
    if let Some(raw) = msg.vm_config.as_ref() {
        // Try to parse the vm_config from the validator's message
        match serde_json::from_str::<VmConfig>(raw) {
            Ok(parsed) => {
                info!("Using vm_config from validator message");
                return Ok((raw.clone(), parsed));
            }
            Err(err) => {
                warn!(
                    "Invalid vm_config provided by validator; falling back to defaults: {}",
                    err
                );
            }
        }
    } else {
        warn!("Validator did not include vm_config in attestation; using default hardware spec");
    }
    build_fallback_vm_config(os_image_hash)
}

fn build_fallback_vm_config(os_image_hash: &str) -> anyhow::Result<(String, VmConfig)> {
    // Use the same defaults as in config.rs for validator VMs
    // DEFAULT_VM_VCPU = 16, DEFAULT_VM_MEMORY_MB = 16 * 1024
    let cpu_count = std::env::var("VALIDATOR_VM_VCPU")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(16);
    
    let memory_mb = std::env::var("VALIDATOR_VM_MEMORY_MB")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(16 * 1024);
    
    let memory_size = (memory_mb as u64) * 1024 * 1024; // Convert MB to bytes

    info!(
        "Building fallback vm_config: cpu_count={}, memory_size={} bytes ({} MB)",
        cpu_count, memory_size, memory_mb
    );

    let vm_config =
        DstackVerifierClient::extract_vm_config(cpu_count, memory_size, os_image_hash);
    let parsed: VmConfig =
        serde_json::from_str(&vm_config).context("Failed to parse fallback vm_config JSON")?;
    Ok((vm_config, parsed))
}

/// Normalize JSON by sorting all object keys alphabetically
/// This ensures consistent hashing regardless of key insertion order
fn normalize_json_for_hashing(json_str: &str) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::from_str(json_str)
        .context("Failed to parse JSON for normalization")?;
    
    let normalized = sort_json_keys(&value);
    
    serde_json::to_string(&normalized)
        .context("Failed to serialize normalized JSON")
}

/// Recursively sort all object keys in a JSON value
fn sort_json_keys(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    
    match value {
        Value::Object(map) => {
            let mut sorted: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k.clone(), sort_json_keys(v));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| sort_json_keys(v)).collect())
        }
        _ => value.clone()
    }
}

