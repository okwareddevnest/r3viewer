use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, migrate::MigrateDatabase};
use std::path::PathBuf;
use tauri::AppHandle;
use anyhow::Result;

pub mod models;
pub mod schema;

pub use models::*;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(app_handle: &AppHandle) -> Result<Self> {
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to resolve app data directory");
        
        std::fs::create_dir_all(&app_dir)?;
        
        let database_path = app_dir.join("r3viewer.db");
        let database_url = format!("sqlite://{}", database_path.display());
        
        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&database_url).await? {
            sqlx::Sqlite::create_database(&database_url).await?;
        }
        
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Database { pool })
    }
    
    pub async fn initialize_schema(&self) -> Result<()> {
        schema::create_tables(&self.pool).await
    }
} 