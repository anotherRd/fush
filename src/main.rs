pub mod helper;
pub mod migration;
pub mod database;
pub mod config;

use clap::{Parser, CommandFactory};
use std::env;
use std::process::{Command};
use crate::helper::{read_from_input, split_selected, connect_to_server, connect_to_container, connect_to_server_container, db_array_placeholders, connect_to_server_args, select_server, select_multi_server, select_nodes, create_key_pair, key_pair_exists};
use crate::config::{key_dir};
use crate::migration::migrate;
use crate::database::get_db_pool;
use sqlx::{Row};
use crate::config::{init_config};
use std::path::Path;
use std::fs;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    connect: bool,
    #[arg(short='S', long)]
    sync_all_server_container: bool,
    #[arg(short, long)]
    scyn_server_container: bool,
    #[arg(short, long)]
    new: bool,
    #[arg(short, long)]
    delete: bool,
    #[arg(short, long)]
    edit: bool,
    #[arg(short, long)]
    key: bool,
}

async fn new_node() -> Result<(), Box<dyn std::error::Error>> {
    // read user input
    println!("Create new server");
    let name = read_from_input("Name", None, vec![], true)?;
    let user = read_from_input("User", None, vec![], true)?;
    let host = read_from_input("Host", None, vec![], true)?;
    let port = read_from_input("Port (22)", Some("22"), vec![], true)?;
    let key_input = read_from_input("Custom key name (use default key/password if empty)", Some(""), vec![], false)?;
    let key = if key_input == "" { None } else { Some(key_input) };

    let port = if port == "" { "22" } else { &port };
    let address = format!("{user}@{host}:{port}");

    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

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
        .bind(&name)
        .bind(&address)
        .bind("server")
        .bind(&key)
        .execute(&mut *tx)
        .await?;

    // generate key
    if let Some(key_value) = &key {
        if !create_key_pair(&key_value, false)? {
            println!("INFO use existing key: {key_value}");
        } else {
            println!("INFO created key: {key_value}");
        }
    }

    tx.commit().await?;
    println!("INFO server created: {name}");

    Ok(())
}

async fn edit_node() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;
    
    // start selection
    let selected = select_server("Select server to edit").await?;
    if selected == "" {
        return Ok(());
    }
    
    // format selected
    let (_prefix, selected_name) = split_selected(&selected);

    // get detail
    let row = sqlx::query("SELECT * FROM nodes WHERE name = ?")
        .bind(&selected_name)
        .fetch_one(&pool)
        .await?;

    let old_id: i64 = row.get("id");
    let old_name: String = row.get("name");
    let old_key: Option<String> = row.get("key");
    let old_node_type: String = row.get("node_type");
    let old_address: String = row.get("address");

    // split address
    let (old_user, rest) = old_address.split_once('@').unwrap();
    let (old_host, old_port) = rest.split_once(':').unwrap();

    // read user input
    println!("Edit server: {selected_name}");
    let name = read_from_input(&format!("Name ({old_name})"), Some(&old_name), vec![], true)?;
    let user = read_from_input(&format!("User ({old_user})"), Some(&old_user), vec![], true)?;
    let host = read_from_input(&format!("Host ({old_host})"), Some(&old_host), vec![], true)?;
    let port = read_from_input(&format!("Port ({old_port})"), Some(&old_port), vec![], true)?;
    let change_key = read_from_input(&format!("Change key ({}) [y/n]?", old_key.as_ref().unwrap()), None, vec!["y", "n"], true)?;

    let key;
    if change_key == "y" {
        let key_input = read_from_input("Custom key name (use default key/password if empty)", Some(""), vec![], false)?;
        key = if key_input == "" { None } else { Some(key_input) };
    } else {
        key = old_key;
    }

    let port = if port == "" { "22" } else { &port };
    let address = format!("{user}@{host}:{port}");

    sqlx::query(
            "UPDATE nodes
                set name = ?,
                address = ?,
                node_type = ?,
                key = ?
            WHERE id = ?"
        )
        .bind(&name)
        .bind(&address)
        .bind(&old_node_type)
        .bind(&key)
        .bind(&old_id)
        .execute(&mut *tx)
        .await?;

    // generate key
    if let Some(key_value) = &key {
        if !create_key_pair(&key_value, false)? {
            println!("INFO use existing key: {key_value}");
        } else {
            println!("INFO created key: {key_value}");
        }
    }

    tx.commit().await?;
    println!("INFO server updated");

    Ok(())
}

async fn delete_node() -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    // start multi selection
    let selections = select_multi_server("Select server(s) to delete").await?;

    // if nothing is selected
    if selections.len() == 0 {
        return Ok(());
    }

    // confirmation
    println!("To be deleted:");
    println!("{:?}", &selections);
    let confirmation = read_from_input("Are you sure [y/n]?", None, vec!["y", "n"], true)?;
    if confirmation == "n" {
        return Ok(());
    }

    // format selected raw 
    let mut selected_names = Vec::new();
    for selected in &selections {
        let (_prefix, selected_name) = split_selected(&selected);
        selected_names.push(selected_name);
    }

    // delete from db
    let query = format!(
        "DELETE FROM nodes WHERE name IN ({})",
        &db_array_placeholders(selected_names.len())
    );

    // bind actual data
    let mut q = sqlx::query(&query);
    for selected_name in &selected_names {
        q = q.bind(selected_name);
    }
    q.execute(&mut *tx).await?;

    tx.commit().await?;
    
    println!("INFO {:?} deleted", {&selections});
    
    Ok(())
}

async fn connect()-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;

    // start selection
    let selected = select_nodes("Select node to connect").await?;

    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }

    println!("Selected: {selected}");
    let (prefix, selected) = split_selected(&selected);

    // connect
    match prefix.as_str() {
        "container" => {
            // execute docker exec
            connect_to_container(&selected, vec![])?;
        },
        "server" => {
            // get from database
            let row = sqlx::query("SELECT * FROM nodes WHERE name = ?")
                .bind(&selected)
                .fetch_one(&pool)
                .await?;
            
            // split address and port
            let key: Option<&str> = row.get("key");
            let address: String = row.get("address");
            
            // execute ssh
            connect_to_server(&key, &address, &vec![])?;
        },
        "server container" => {
            // get from database
            let row = sqlx::query("SELECT 
                    nodes.*,
                    server.key as server_key,
                    server.address as server_address
                    FROM nodes
                    JOIN nodes AS server ON nodes.parent_id = server.id
                    WHERE nodes.name = ?
                ")
                .bind(&selected)
                .fetch_one(&pool)
                .await?;
            
            // split address and port
            let container_address: String = row.get("address");
            let server_key: Option<&str> = row.get("server_key");
            let server_address: String = row.get("server_address");
            
            // execute ssh
            connect_to_server_container(&container_address, &server_key, &server_address, vec!["-t"])?;
        },
        _ => ()
    }

    Ok(())
}

async fn scyn_server_container(scan_all: bool)-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;
    let rows;

    // select nodes if scan all is false
    if !scan_all {
        // start multi selection
        let selections = select_multi_server("Select server(s) to scan").await?;

        // if nothing is selected
        if selections.len() == 0 {
            return Ok(());
        }

        // format selected raw 
        let mut selected_names = Vec::new();
        for selected in selections {
            let (_prefix, selected) = split_selected(&selected);
            selected_names.push(selected);
        }

        // get selected data
        let query = format!(
            "SELECT * FROM nodes WHERE node_type = 'server' and name IN ({})",
            &db_array_placeholders(selected_names.len())
        );

        // bind actual data
        let mut q = sqlx::query(&query);
        for selected_name in &selected_names {
            q = q.bind(selected_name);
        }
        rows = q.fetch_all(&mut *tx).await?;
    } else {
        rows = sqlx::query("SELECT * FROM nodes WHERE node_type = 'server'")
            .fetch_all(&pool)
            .await?;
    }

    // scanning
    for row in rows {
        let id: i64 = row.get("id");
        let name: String = row.get("name");
        let key: Option<&str> = row.get("key");
        let address: String = row.get("address");

        println!("Scanning container on : {name}");
        
        // ssh args
        let mut ssh_args = connect_to_server_args(&key, &address, &vec![])?;
        ssh_args.extend(["-t".to_string(), "docker ps --format {{.Names}}".to_string()]);

        // execute ssh
        let output = Command::new("ssh")
            .args(ssh_args)
            .output()?;

        // turn it to vec
        let containers: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();

        // delete server container on current node
        sqlx::query(
                    "DELETE FROM nodes WHERE parent_id = ?"
                )
                .bind(id)
                .execute(&mut *tx)
                .await?;

        for container in containers {
            let node_name = format!("{name}: {container}");
            sqlx::query(
                    "INSERT INTO nodes (
                        name,
                        address,
                        node_type,
                        parent_id
                    ) VALUES (
                        ?,
                        ?,
                        ?,
                        ?
                    )"
                )
                .bind(&node_name)
                .bind(&container)
                .bind("server_container")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;

    Ok(())
}

async fn show_key()-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;

    // start selection
    let selected = select_server("Select server to show key").await?;

    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }

    println!("Selected: {selected}");
    let (_prefix, selected_name) = split_selected(&selected);

    // get detail
    let row = sqlx::query("SELECT * FROM nodes WHERE name = ?")
        .bind(&selected_name)
        .fetch_one(&pool)
        .await?;

    let key: Option<&str> = row.get("key");
    if let Some(key_value) = key {
        if key_pair_exists(&key_value)? {
            let key_dir = key_dir()?;
            let key_path = format!("{}/{}.pub", &key_dir, &key_value);
            let key_content = fs::read_to_string(&key_path)?;
            println!("Key path: {key_path}");
            println!("{key_content}");
        } else {
            eprintln!("ERROR Key {key_value} not found");
        }
    } else {
        println!("Use default keys");
    }


    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // init config
    let _ = init_config()?;

    // migrate db
    let _ = migrate().await?;

    // args not provided
    if env::args().len() == 1 {
        Args::command().print_help()?;
    }

    // get args
    let args = Args::parse();

    if args.connect {
        connect().await?;
    } else if args.new {
        new_node().await?;
    } else if args.edit {
        edit_node().await?;
    } else if args.delete {
        delete_node().await?;
    } else if args.sync_all_server_container {
        scyn_server_container(true).await?;
    } else if args.scyn_server_container {
        scyn_server_container(false).await?;
    } else if args.key {
        show_key().await?;
    }

    Ok(())
}