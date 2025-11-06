// Integration tests for Job Flow
// Tests: Create job → Distribute → Execute → Complete
// Uses: Real PostgreSQL, Real Redis (fakeredis), Mock: Validator (mock HTTP server)

use platform_api_scheduler::{SchedulerService, SchedulerConfig, CreateJobRequest};
use platform_api_models::{JobStatus, JobPriority, RuntimeType, Id, ClaimJobRequest, SubmitResultRequest, EvalResult, ResourceUsage};
use sqlx::PgPool;
use uuid::Uuid;
use serde_json::json;
use std::sync::Arc;
use std::collections::BTreeMap;
use std::path::PathBuf;

// Helper to create test database pool
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://platform:platform@localhost:5432/platform_test".to_string());
    
    let pool = PgPool::connect(&database_url).await
        .expect("Failed to connect to test database");
    
    // Run migrations
    let migrations_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("crates/storage/migrations"))
        .expect("Failed to find migrations directory");
    
    sqlx::migrate::Migrator::new(&migrations_path)
        .await
        .expect("Failed to create migrator")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    
    pool
}

// Helper to cleanup test data
async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("DELETE FROM job_test_results").execute(pool).await.ok();
    sqlx::query("DELETE FROM jobs").execute(pool).await
        .expect("Failed to cleanup test data");
}

#[tokio::test]
#[ignore] // Requires database setup
async fn test_full_job_flow() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    
    // 1. Create job
    let request = CreateJobRequest {
        challenge_id: Id::from(challenge_id),
        payload: json!({"test": "data"}),
        priority: Some(JobPriority::Normal),
        runtime: RuntimeType::Docker,
        timeout: Some(3600),
        max_retries: Some(3),
    };
    
    let job = scheduler.create_job(request).await
        .expect("Failed to create job");
    
    assert_eq!(job.status, JobStatus::Pending);
    
    // 2. Claim job (simulating validator)
    let claim_request = ClaimJobRequest {
        validator_hotkey: "test-validator".to_string().into(),
        runtime: RuntimeType::Docker,
        capabilities: vec![],
    };
    
    let claimed = scheduler.claim_job(claim_request).await
        .expect("Failed to claim job");
    
    assert_eq!(claimed.job.status, JobStatus::Claimed);
    
    // 3. Complete job (simulating validator submitting results)
    let eval_result = EvalResult {
        job_id: job.id,
        submission_id: Id::from(Uuid::new_v4()),
        scores: {
            let mut scores = BTreeMap::new();
            scores.insert("overall".to_string(), 0.95);
            scores
        },
        metrics: {
            let mut metrics = BTreeMap::new();
            metrics.insert("accuracy".to_string(), 0.95);
            metrics
        },
        logs: vec!["Test log".to_string()],
        error: None,
        execution_time: 1000,
        resource_usage: ResourceUsage {
            cpu_time: 500,
            memory_peak: 1024,
            disk_usage: 2048,
            network_bytes: 512,
        },
        attestation_receipt: None,
    };
    
    let complete_request = SubmitResultRequest {
        job_id: job.id,
        result: eval_result,
        receipts: vec![],
    };
    
    scheduler.complete_job(job.id.into(), complete_request).await
        .expect("Failed to complete job");
    
    // 4. Verify job is completed
    let completed_job = scheduler.get_job(job.id.into()).await
        .expect("Failed to get job");
    
    assert_eq!(completed_job.status, JobStatus::Completed);
    
    cleanup_test_data(&pool).await;
}

