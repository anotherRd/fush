use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use crate::config::db_file;

pub async fn get_db_pool() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let db_file = db_file()?;
    let db_url = format!("sqlite://{}", db_file.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    Ok(pool)
}