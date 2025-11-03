use platform_api_models::*;
use uuid::Uuid;

mod capacity;
pub use capacity::*;

mod scoring;
pub use scoring::*;

/// Scheduler service
pub struct SchedulerService {
    config: SchedulerConfig,
    jobs: tokio::sync::RwLock<std::collections::HashMap<Uuid, JobMetadata>>,
}

impl SchedulerService {
    pub fn new(config: &SchedulerConfig) -> std::result::Result<Self, anyhow::Error> {
        Ok(Self {
            config: config.clone(),
            jobs: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        })
    }

    pub async fn list_jobs(&self, _page: u32, _per_page: u32, _status: Option<String>, _challenge_id: Option<Uuid>) -> std::result::Result<JobListResponse, anyhow::Error> {
        let jobs = self.jobs.read().await;
        Ok(JobListResponse {
            jobs: jobs.values().cloned().collect(),
            total: jobs.len() as u64,
            page: 1,
            per_page: 20,
        })
    }

    pub async fn get_job(&self, id: Uuid) -> std::result::Result<JobMetadata, anyhow::Error> {
        let jobs = self.jobs.read().await;
        jobs.get(&id).cloned().ok_or_else(|| anyhow::anyhow!("Job not found"))
    }

    pub async fn claim_job(&self, _request: ClaimJobRequest) -> std::result::Result<ClaimJobResponse, anyhow::Error> {
        Err(anyhow::anyhow!("No jobs available"))
    }

    pub async fn claim_specific_job(&self, _job_id: Uuid, _request: ClaimJobRequest) -> std::result::Result<ClaimJobResponse, anyhow::Error> {
        Err(anyhow::anyhow!("Job not available"))
    }

    pub async fn complete_job(&self, _job_id: Uuid, _result: SubmitResultRequest) -> std::result::Result<(), anyhow::Error> {
        Ok(())
    }

    pub async fn fail_job(&self, _job_id: Uuid, _request: FailJobRequest) -> std::result::Result<(), anyhow::Error> {
        Ok(())
    }

    pub async fn get_next_job(&self, _validator_hotkey: String, _runtime: Option<String>) -> std::result::Result<Option<ClaimJobResponse>, anyhow::Error> {
        Ok(None)
    }

    pub async fn get_job_stats(&self) -> std::result::Result<JobStats, anyhow::Error> {
        Ok(JobStats {
            total_jobs: 0,
            pending_jobs: 0,
            running_jobs: 0,
            completed_jobs: 0,
            failed_jobs: 0,
            avg_execution_time: 0.0,
            success_rate: 0.0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub max_concurrent_jobs: u32,
    pub job_timeout: u64,
    pub retry_attempts: u32,
    pub retry_delay: u64,
    pub cleanup_interval: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_jobs: 100,
            job_timeout: 3600,
            retry_attempts: 3,
            retry_delay: 60,
            cleanup_interval: 3600,
        }
    }
}

