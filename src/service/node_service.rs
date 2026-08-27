use crate::dto::node_dto::ParentNodeDto;
use crate::{dto::node_dto::NodeDto, service_params::node_service_params::EditServerServiceParams};
use crate::service_params::node_service_params::AddServerServiceParams;
use crate::database::get_db_pool;
use crate::helper::{create_key_pair, custom_print, db_array_placeholders};
use sqlx::{Row};

pub async fn find_node_by_name(name: &str) -> Result<NodeDto, Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let row = sqlx::query("SELECT 
            nodes.id,
            nodes.name,
            nodes.address,
            nodes.node_type,
            nodes.parent_id,
            nodes.key,
            parent.id as parent_id,
            parent.name as parent_name,
            parent.address as parent_address,
            parent.node_type as parent_node_type,
            parent.parent_id as parent_parent_id,
            parent.key as parent_key
        FROM nodes 
        LEFT JOIN nodes AS parent ON nodes.parent_id = parent.id
        WHERE nodes.name = ?")
        .bind(&name)
        .fetch_one(&pool)
        .await?;

    let id: i64 = row.get("id");
    let name: String = row.get("name");
    let address: String = row.get("address");
    let node_type: String = row.get("node_type");
    let parent_id: Option<i64> = row.get("parent_id");
    let key: Option<String> = row.get("key");

    let mut parent: Option<ParentNodeDto> = None;
    let check_parent: Option<i64> = row.get("parent_id");
    if check_parent.is_some() {
        let parent_id: i64 = row.get("parent_id");
        let parent_name: String = row.get("parent_name");
        let parent_address: String = row.get("parent_address");
        let parent_node_type: String = row.get("parent_node_type");
        let parent_parent_id: Option<i64> = row.get("parent_parent_id");
        let parent_key: Option<String> = row.get("parent_key");
        parent = Some(ParentNodeDto {
            id: parent_id,
            name: parent_name,
            address: parent_address,
            node_type: parent_node_type,
            parent_id: parent_parent_id,
            key: parent_key,
        });
    }

    Ok(NodeDto{
        id: id,
        name: name,
        address: address,
        node_type: node_type,
        parent_id: parent_id,
        key: key,
        parent: parent,
    })
}

pub async fn get_node_by_names(name: &str) -> Result<Vec<NodeDto>, Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let rows = sqlx::query("SELECT 
            nodes.id,
            nodes.name,
            nodes.address,
            nodes.node_type,
            nodes.parent_id,
            nodes.key,
            parent.id as parent_id,
            parent.name as parent_name,
            parent.address as parent_address,
            parent.node_type as parent_node_type,
            parent.parent_id as parent_parent_id,
            parent.key as parent_key
        FROM nodes 
        LEFT JOIN nodes AS parent ON nodes.parent_id = parent.id
        WHERE nodes.name = ?")
        .bind(&name)
        .fetch_all(&pool)
        .await?;

    let mut node_dtos = Vec::new();
    for row in rows {
        let id: i64 = row.get("id");
        let name: String = row.get("name");
        let address: String = row.get("address");
        let node_type: String = row.get("node_type");
        let parent_id: Option<i64> = row.get("parent_id");
        let key: Option<String> = row.get("key");

        let mut parent: Option<ParentNodeDto> = None;
        let check_parent: Option<i64> = row.get("parent_id");
        if check_parent.is_some() {
            let parent_id: i64 = row.get("parent_id");
            let parent_name: String = row.get("parent_name");
            let parent_address: String = row.get("parent_address");
            let parent_node_type: String = row.get("parent_node_type");
            let parent_parent_id: Option<i64> = row.get("parent_parent_id");
            let parent_key: Option<String> = row.get("parent_key");
            parent = Some(ParentNodeDto {
                id: parent_id,
                name: parent_name,
                address: parent_address,
                node_type: parent_node_type,
                parent_id: parent_parent_id,
                key: parent_key,
            });
        }

        node_dtos.push(NodeDto{
            id: id,
            name: name,
            address: address,
            node_type: node_type,
            parent_id: parent_id,
            key: key,
            parent: parent,
        })
    }

    Ok(node_dtos)
}

pub async fn add_server(params: AddServerServiceParams) -> Result<(), Box<dyn std::error::Error>> {
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
        if !create_key_pair(&key_value, false, params.default_passphrase)? {
            custom_print("info", &format!("use existing key: {key_value}"));
        } else {
            custom_print("info", &format!("new key created: {key_value}"));
        }
    }

    tx.commit().await?;

    custom_print("success", &format!("server added"));

    Ok(())
}

pub async fn edit_server(id: i64, params: EditServerServiceParams) -> Result<(), Box<dyn std::error::Error>> {
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
        if !create_key_pair(&key_value, false, params.default_passphrase)? {
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

pub async fn delete_all() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    // delete from db
    let query = format!("DELETE FROM nodes");

    // bind actual data
    let q = sqlx::query(&query);
    q.execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(())
}

pub fn get_blacklisted_key_name<'a>() -> Vec<&'a str> {
    return vec![
        "config",
        "known_hosts",
        "known_hosts.old",
        "authorized_keys",
        "authorized_keys2",
        "environment",
        "rc",
    ];
}