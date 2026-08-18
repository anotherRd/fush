use crate::database::get_db_pool;

pub async fn migrate() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;
    
    // create nodes
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS nodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            address TETXT,
            auth_type TEXT,
            node_type TEXT,
            parent_id INTEGER,
            FOREIGN KEY (parent_id) REFERENCES nodes(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    
    Ok(())
}