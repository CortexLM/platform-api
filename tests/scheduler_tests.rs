// Unit tests for Scheduler Service
// Uses real PostgreSQL with sqlx::test (fast, testable)

use platform_api_scheduler::{SchedulerService, SchedulerConfig, CreateJobRequest};
use platform_api_models::{
    JobStatus, JobPriority, RuntimeType, Id, ClaimJobRequest, SubmitResultRequest, 
    FailJobRequest, EvalResult, ResourceUsage, Hotkey
};
use sqlx::PgPool;
use uuid::Uuid;
use serde_json::json;
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::BTreeMap;

// Helper to create test database pool
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://platform:platform@localhost:5432/platform_test".to_string());
    
    let pool = PgPool::connect(&database_url).await
        .expect("Failed to connect to test database");
    
    // Run migrations - find migrations directory relative to workspace root
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
async fn test_create_job() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
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
    assert_eq!(job.priority, JobPriority::Normal);
    assert_eq!(job.runtime, RuntimeType::Docker);
    assert_eq!(job.retry_count, 0);
    assert_eq!(job.max_retries, 3);
    assert!(job.timeout_at.is_some());
    
    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_create_job_with_priority() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    let request = CreateJobRequest {
        challenge_id: Id::from(challenge_id),
        payload: json!({"test": "data"}),
        priority: Some(JobPriority::High),
        runtime: RuntimeType::Docker,
        timeout: None,
        max_retries: None,
    };
    
    let job = scheduler.create_job(request).await
        .expect("Failed to create job");
    
    assert_eq!(job.priority, JobPriority::High);
    assert_eq!(job.max_retries, 3); // Default value
    
    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_list_jobs() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    
    // Create multiple jobs
    for i in 0..5 {
        let request = CreateJobRequest {
            challenge_id: Id::from(challenge_id),
            payload: json!({"index": i}),
            priority: Some(JobPriority::Normal),
            runtime: RuntimeType::Docker,
            timeout: None,
            max_retries: None,
        };
        scheduler.create_job(request).await.expect("Failed to create job");
    }
    
    // List all jobs
    let response = scheduler.list_jobs(1, 10, None, None).await
        .expect("Failed to list jobs");
    
    assert_eq!(response.jobs.len(), 5);
    assert_eq!(response.total, 5);
    assert_eq!(response.page, 1);
    assert_eq!(response.per_page, 10);
    
    // List jobs with pagination
    let response = scheduler.list_jobs(1, 2, None, None).await
        .expect("Failed to list jobs");
    
    assert_eq!(response.jobs.len(), 2);
    assert_eq!(response.total, 5);
    
    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_list_jobs_with_status_filter() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    
    // Create jobs
    for _ in 0..3 {
        let request = CreateJobRequest {
            challenge_id: Id::from(challenge_id),
            payload: json!({}),
            priority: Some(JobPriority::Normal),
            runtime: RuntimeType::Docker,
            timeout: None,
            max_retries: None,
        };
        scheduler.create_job(request).await.expect("Failed to create job");
    }
    
    // List pending jobs
    let response = scheduler.list_jobs(1, 10, Some("pending".to_string()), None).await
        .expect("Failed to list jobs");
    
    assert_eq!(response.jobs.len(), 3);
    for job in &response.jobs {
        assert_eq!(job.status, JobStatus::Pending);
    }
    
    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_claim_job() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    let request = CreateJobRequest {
        challenge_id: Id::from(challenge_id),
        payload: json!({}),
        priority: Some(JobPriority::Normal),
        runtime: RuntimeType::Docker,
        timeout: None,
        max_retries: None,
    };
    
    let job = scheduler.create_job(request).await
        .expect("Failed to create job");
    
    let claim_request = ClaimJobRequest {
        validator_hotkey: Hotkey::from("test-validator".to_string()),
        runtime: RuntimeType::Docker,
        capabilities: vec![],
    };
    
    let claimed = scheduler.claim_job(claim_request).await
        .expect("Failed to claim job");
    
    assert_eq!(claimed.job.id, job.id);
    assert_eq!(claimed.job.status, JobStatus::Claimed);
    
    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_complete_job() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    let request = CreateJobRequest {
        challenge_id: Id::from(challenge_id),
        payload: json!({}),
        priority: Some(JobPriority::Normal),
        runtime: RuntimeType::Docker,
        timeout: None,
        max_retries: None,
    };
    
    let job = scheduler.create_job(request).await
        .expect("Failed to create job");
    
    // Claim the job first
    let claim_request = ClaimJobRequest {
        validator_hotkey: Hotkey::from("test-validator".to_string()),
        runtime: RuntimeType::Docker,
        capabilities: vec![],
    };
    scheduler.claim_job(claim_request).await.expect("Failed to claim job");
    
    // Complete the job with EvalResult
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
    
    // Verify job is completed
    let completed_job = scheduler.get_job(job.id.into()).await
        .expect("Failed to get job");
    
    assert_eq!(completed_job.status, JobStatus::Completed);
    
    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn test_retry_logic() {
    let pool = setup_test_db().await;
    cleanup_test_data(&pool).await;
    
    let config = SchedulerConfig::default();
    let scheduler = SchedulerService::with_database(&config, Arc::new(pool.clone()))
        .expect("Failed to create scheduler");
    
    let challenge_id = Uuid::new_v4();
    let request = CreateJobRequest {
        challenge_id: Id::from(challenge_id),
        payload: json!({}),
        priority: Some(JobPriority::Normal),
        runtime: RuntimeType::Docker,
        timeout: None,
        max_retries: Some(2),
    };
    
    let job = scheduler.create_job(request).await
        .expect("Failed to create job");
    
    // Fail the job
    let fail_request = FailJobRequest {
        reason: "Test failure".to_string(),
        error_details: Some("Test error details".to_string()),
    };
    
    scheduler.fail_job(job.id.into(), fail_request).await
        .expect("Failed to fail job");
    
    // Verify job is failed
    let failed_job = scheduler.get_job(job.id.into()).await
        .expect("Failed to get job");
    
    assert_eq!(failed_job.status, JobStatus::Failed);
    assert_eq!(failed_job.retry_count, 0); // First failure
    
    cleanup_test_data(&pool).await;
}
