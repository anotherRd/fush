use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use crate::config::db_file;

pub async fn get_db_pool() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let db_file = db_file()?;
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_file)
        .await?;

    Ok(pool)
}