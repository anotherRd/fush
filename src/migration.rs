use crate::database::get_db_pool;

pub async fn migrate() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    sqlx::migrate!()
        .run(&pool)
        .await?;

    Ok(())
}