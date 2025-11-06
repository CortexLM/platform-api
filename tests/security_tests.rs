// Unit tests for Security module
// Tests compose hash calculation and key derivation

use platform_api::security::PlatformSecurity;
use platform_api::compose_hash;
use sha2::{Sha256, Digest};
use hex;

#[test]
fn test_compose_hash_calculation() {
    // Create a test docker-compose content
    let test_compose = r#"
version: '3.8'
services:
  api:
    image: nginx:latest
    ports:
      - "3000:3000"
"#;
    
    // Write to temporary file
    let temp_file = std::env::temp_dir().join("test-docker-compose.yml");
    std::fs::write(&temp_file, test_compose).expect("Failed to write test file");
    
    // Calculate hash
    let hash = platform_api::compose_hash::calculate_compose_hash(
        temp_file.to_str().unwrap()
    ).expect("Failed to calculate compose hash");
    
    // Hash should be a valid hex string (64 chars for SHA256)
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    
    // Cleanup
    std::fs::remove_file(&temp_file).ok();
}

#[test]
fn test_compose_hash_consistency() {
    // Same content should produce same hash
    let test_compose = "version: '3.8'\nservices:\n  api:\n    image: nginx\n";
    
    let temp_file1 = std::env::temp_dir().join("test-compose-1.yml");
    let temp_file2 = std::env::temp_dir().join("test-compose-2.yml");
    
    std::fs::write(&temp_file1, test_compose).expect("Failed to write test file");
    std::fs::write(&temp_file2, test_compose).expect("Failed to write test file");
    
    let hash1 = platform_api::compose_hash::calculate_compose_hash(
        temp_file1.to_str().unwrap()
    ).expect("Failed to calculate hash");
    
    let hash2 = platform_api::compose_hash::calculate_compose_hash(
        temp_file2.to_str().unwrap()
    ).expect("Failed to calculate hash");
    
    assert_eq!(hash1, hash2, "Same content should produce same hash");
    
    // Cleanup
    std::fs::remove_file(&temp_file1).ok();
    std::fs::remove_file(&temp_file2).ok();
}

#[test]
fn test_security_key_derivation() {
    let compose_hash = "test-compose-hash-12345";
    
    let security = PlatformSecurity::new_with_compose_hash(compose_hash)
        .expect("Failed to create PlatformSecurity");
    
    // Verify compose hash is stored
    assert_eq!(security.get_compose_hash(), compose_hash);
    
    // Verify public key is generated
    let public_key = security.get_public_key();
    assert_eq!(public_key.len(), 32); // Ed25519 public key is 32 bytes
    
    // Verify signing works
    let message = b"test message";
    let signature = security.sign(message);
    assert_eq!(signature.len(), 64); // Ed25519 signature is 64 bytes
}

#[test]
fn test_security_deterministic_keys() {
    // Same compose hash should produce same keys
    let compose_hash = "test-compose-hash-deterministic";
    
    let security1 = PlatformSecurity::new_with_compose_hash(compose_hash)
        .expect("Failed to create PlatformSecurity");
    
    let security2 = PlatformSecurity::new_with_compose_hash(compose_hash)
        .expect("Failed to create PlatformSecurity");
    
    // Same compose hash should produce same public key
    assert_eq!(
        security1.get_public_key(),
        security2.get_public_key(),
        "Same compose hash should produce same public key"
    );
    
    // Same message should produce same signature
    let message = b"test message";
    let sig1 = security1.sign(message);
    let sig2 = security2.sign(message);
    
    assert_eq!(sig1, sig2, "Same compose hash and message should produce same signature");
}

#[test]
fn test_signed_header_creation() {
    let compose_hash = "test-compose-hash";
    let security = PlatformSecurity::new_with_compose_hash(compose_hash)
        .expect("Failed to create PlatformSecurity");
    
    let timestamp = 1234567890;
    let nonce = "test-nonce";
    
    let header = security.create_signed_header(timestamp, nonce);
    
    // Header should contain signature and message
    assert!(header.contains(':'));
    let parts: Vec<&str> = header.split(':').collect();
    assert_eq!(parts.len(), 3); // signature:timestamp:nonce
    
    // Signature should be hex-encoded (128 chars for 64 bytes)
    assert_eq!(parts[0].len(), 128);
}

