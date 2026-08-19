use crate::{dto::node_dto::NodeDto, service_params::node_service_params::EditNodeServiceParams};
use crate::service_params::node_service_params::NewNodeServiceParams;
use crate::database::get_db_pool;
use crate::helper::{create_key_pair, custom_print, db_array_placeholders};
use sqlx::{Row};

pub async fn get_server_by_name(name: &str) -> Result<NodeDto, Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let row = sqlx::query("SELECT * FROM nodes WHERE name = ?")
        .bind(&name)
        .fetch_one(&pool)
        .await?;

    let id: i64 = row.get("id");
    let name: String = row.get("name");
    let address: String = row.get("address");
    let node_type: String = row.get("node_type");
    let parent_id: Option<i64> = row.get("parent_id");
    let key: Option<String> = row.get("key");

    Ok(NodeDto{
        id: id,
        name: name,
        address: address,
        node_type: node_type,
        parent_id: parent_id,
        key: key,
    })
}

pub async fn add_server(params: NewNodeServiceParams) -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    let key = if params.key == "" { None } else { Some(params.key) };
    let port = if params.port == "" { "22" } else { &params.port };
    let address = format!("{}@{}:{}", &params.user, &params.host, &port);

    sqlx::query(
            "INSERT INTO nodes (
                name,
                address,
                node_type,
                key
            ) VALUES (
                ?, 
                ?,
                ?,
                ?
            )"
        )
        .bind(&params.name)
        .bind(&address)
        .bind("server")
        .bind(&key)
        .execute(&mut *tx)
        .await?;

    // generate key
    if let Some(key_value) = &key {
        if !create_key_pair(&key_value, false)? {
            custom_print("info", &format!("use existing key: {key_value}"));
        } else {
            custom_print("info", &format!("new key created: {key_value}"));
        }
    }

    tx.commit().await?;

    custom_print("success", &format!("server added"));

    Ok(())
}

pub async fn edit_server(id: i64, params: EditNodeServiceParams) -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    let key = if params.key == "" { None } else { Some(params.key) };
    let port = if params.port == "" { "22" } else { &params.port };
    let address = format!("{}@{}:{}", &params.user, &params.host, &port);

    sqlx::query(
            "UPDATE nodes
                set name = ?,
                address = ?,
                key = ?
            WHERE id = ?"
        )
        .bind(&params.name)
        .bind(&address)
        .bind(&key)
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // generate key
    if let Some(key_value) = &key {
        if !create_key_pair(&key_value, false)? {
            custom_print("info", &format!("use existing key: {key_value}"));
        } else {
            custom_print("info", &format!("new key created: {key_value}"));
        }
    }

    tx.commit().await?;
    
    custom_print("success", &format!("server edited"));

    Ok(())
}

pub async fn delete_server_by_names(names: &Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    // delete from db
    let query = format!(
        "DELETE FROM nodes WHERE name IN ({})",
        &db_array_placeholders(names.len())
    );

    // bind actual data
    let mut q = sqlx::query(&query);
    for selected_name in names {
        q = q.bind(selected_name);
    }
    q.execute(&mut *tx).await?;

    tx.commit().await?;

    custom_print("success", &format!("server: {:?} deleted", {&names}));

    Ok(())
}