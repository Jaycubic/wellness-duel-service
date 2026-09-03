use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;

pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
    let db_name = env::var("DB_NAME").unwrap_or_else(|_| "wellness_duel".into());
    let db_user = env::var("DB_USER").unwrap_or_else(|_| "postgres".into());
    let db_password = env::var("DB_PASSWORD").unwrap_or_default();
    let db_host = env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let db_port: u16 = env::var("DB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(5432);

    let url = format!(
        "postgres://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}"
    );

    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(60))
        .connect(&url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
