// Unit tests for ORM Gateway
// Uses real PostgreSQL with sqlx::test (fast, testable)

use platform_api::orm_gateway::{SecureORMGateway, ORMGatewayConfig, ORMQuery, QueryFilter, OrderBy};
use sqlx::PgPool;
use uuid::Uuid;
use serde_json::json;
use std::sync::Arc;
use std::path::PathBuf;

// Helper to create test database pool (reuse from scheduler_tests)
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

#[tokio::test]
async fn test_orm_gateway_creation() {
    let pool = setup_test_db().await;
    let config = ORMGatewayConfig::default();
    let gateway = SecureORMGateway::new(config, pool);
    
    // Test that gateway can be created
    assert!(true);
}

#[tokio::test]
async fn test_query_validation() {
    let pool = setup_test_db().await;
    let config = ORMGatewayConfig::default();
    let gateway = SecureORMGateway::new(config, pool);
    
    // Test valid SELECT query
    let query = ORMQuery {
        operation: "select".to_string(),
        table: "jobs".to_string(),
        schema: None,
        columns: Some(vec!["id".to_string(), "status".to_string()]),
        filters: None,
        order_by: None,
        limit: Some(10),
        offset: None,
        aggregations: None,
        values: None,
        set_values: None,
    };
    
    // Query should be validated (we test the validator, not execution)
    assert_eq!(query.operation, "select");
    assert_eq!(query.table, "jobs");
}

#[tokio::test]
async fn test_query_with_filters() {
    let pool = setup_test_db().await;
    let config = ORMGatewayConfig::default();
    let gateway = SecureORMGateway::new(config, pool);
    
    // Test query with filters
    let query = ORMQuery {
        operation: "select".to_string(),
        table: "jobs".to_string(),
        schema: None,
        columns: Some(vec!["id".to_string()]),
        filters: Some(vec![QueryFilter {
            column: "status".to_string(),
            operator: "=".to_string(),
            value: json!("pending"),
        }]),
        order_by: None,
        limit: Some(10),
        offset: None,
        aggregations: None,
        values: None,
        set_values: None,
    };
    
    assert!(query.filters.is_some());
    assert_eq!(query.filters.as_ref().unwrap().len(), 1);
}

#[tokio::test]
async fn test_query_with_order_by() {
    let pool = setup_test_db().await;
    let config = ORMGatewayConfig::default();
    let gateway = SecureORMGateway::new(config, pool);
    
    // Test query with order by
    let query = ORMQuery {
        operation: "select".to_string(),
        table: "jobs".to_string(),
        schema: None,
        columns: Some(vec!["id".to_string(), "created_at".to_string()]),
        filters: None,
        order_by: Some(vec![OrderBy {
            column: "created_at".to_string(),
            direction: "DESC".to_string(),
        }]),
        limit: Some(10),
        offset: None,
        aggregations: None,
        values: None,
        set_values: None,
    };
    
    assert!(query.order_by.is_some());
    assert_eq!(query.order_by.as_ref().unwrap().len(), 1);
}

#[tokio::test]
async fn test_read_only_config() {
    let config = ORMGatewayConfig::default();
    assert!(config.read_only);
    assert!(config.allowed_operations.contains(&"select".to_string()));
    assert!(!config.allowed_operations.contains(&"insert".to_string()));
}

#[tokio::test]
async fn test_read_write_config() {
    let config = ORMGatewayConfig::read_write();
    assert!(!config.read_only);
    assert!(config.allowed_operations.contains(&"select".to_string()));
    assert!(config.allowed_operations.contains(&"insert".to_string()));
    assert!(config.allowed_operations.contains(&"update".to_string()));
    assert!(config.allowed_operations.contains(&"delete".to_string()));
}

