use crate::database::get_db_pool;
use sqlx::Row;

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
    
    // add default key to nodes
    if !column_exists("nodes", "default_key").await? {
        sqlx::query(r#"
            ALTER TABLE nodes ADD COLUMN default_key BOOL;
            "#,
        )
        .execute(&mut *tx)
        .await?;
    }
    
    // delete auth type from nodes
    if column_exists("nodes", "auth_type").await? {
        sqlx::query(r#"
            ALTER TABLE nodes DROP COLUMN auth_type;
            "#,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    
    Ok(())
}

pub async fn column_exists(table: &str, column: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let row = sqlx::query(r#"
        SELECT EXISTS(
            SELECT 1
            FROM pragma_table_info(?)
            WHERE name = ?
        )
        "#,
    )
    .bind(&table)
    .bind(&column)
    .fetch_one(&pool)
    .await?;

    let exists: bool = row.get(0);
    Ok(exists)
}