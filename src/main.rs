pub mod helper;
pub mod migration;
pub mod database;
pub mod config;

use clap::{Parser, CommandFactory};
use std::env;
use std::process::{Command};
use crate::helper::{read_from_input, split_selected, connect_to_server, connect_to_container, connect_to_server_container, db_array_placeholders, connect_to_server_args, select_server, select_multi_server, select_nodes};
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
    scan_all_server_container: bool,
    #[arg(short, long)]
    scan_server_container: bool,
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
    let name = read_from_input("Name", None, vec![])?;
    let user = read_from_input("User", None, vec![])?;
    let host = read_from_input("Host", None, vec![])?;
    let port = read_from_input("Port (22)", Some("22"), vec![])?;
    let auth_type_choice = read_from_input("Auth Type [(k)ey/(p)assword]?", None, vec!["k", "p"])?;

    let auth_type;
    let mut generate_key = "n".to_string();
    let mut default_key = false;
    match auth_type_choice.as_str() {
        // key
        "k" => {
            auth_type = "key";
            generate_key = read_from_input("Generate new key (use default key if no) [y/n]?", None, vec!["y", "n"])?;
            if generate_key == "n" {
                default_key = true;
            }
        },
        // password
        "p" => {
            auth_type = "password";
        }
        _ => {
            auth_type = "";
        }
    }

    let port = if port == "" { "22" } else { &port };
    let address = format!("{user}@{host}:{port}");

    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    sqlx::query(
            "INSERT INTO nodes (
                name,
                address,
                node_type,
                auth_type,
                default_key
            ) VALUES (
                ?, 
                ?,
                ?,
                ?,
                ?
            )"
        )
        .bind(&name)
        .bind(&address)
        .bind("server")
        .bind(&auth_type)
        .bind(&default_key)
        .execute(&mut *tx)
        .await?;

    // generate key for this node
    if generate_key == "y" {
        let key_dir = key_dir()?;
        let mut overwrite_key = true;

        // check if key exists and overwrite key confirmation
        let key_path = format!("{}/{}", &key_dir, &name);
        let pub_key_path = format!("{}/{}.pub", &key_dir, &name);
        if Path::new(&key_path).exists() && Path::new(&pub_key_path).exists() {
            let overwrite_key_confirmation = read_from_input(&format!("Key already exsits, overwrite the key? [y/n]?"), None, vec!["y", "n"])?;
            if overwrite_key_confirmation == "n" {
                overwrite_key = false;
            } else {
                // delete existing keys
                fs::remove_file(&key_path)?;
                fs::remove_file(&pub_key_path)?;
            }
        }

        if overwrite_key {
            let key_location = format!("{}/{}", &key_dir, name);
            Command::new("ssh-keygen")
                .args(["-f", &key_location])
                .output()?;
        }
    }

    tx.commit().await?;

    Ok(())
}

async fn delete_node() -> Result<(), Box<dyn std::error::Error>> {
    let key_dir = key_dir()?;

    let mut to_be_deleted_keys: Vec<String> = Vec::new();
    let mut delete_all_keys = "".to_string();

    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    // start multi selection
    let selections = select_multi_server().await?;

    // if nothing is selected
    if selections.len() == 0 {
        return Ok(());
    }

    // confirmation
    println!("To be deleted:");
    println!("{:?}", &selections);
    let confirmation = read_from_input("Are you sure [y/n]?", None, vec!["y", "n"])?;
    if confirmation == "n" {
        return Ok(());
    }

    // format selected raw 
    let mut selected_names = Vec::new();
    for selected in selections {
        let (_prefix, selected_name) = split_selected(&selected);
        
        // check if key exists and delete key confirmation
        let key_path = format!("{}/{}", &key_dir, &selected_name);
        let pub_key_path = format!("{}/{}.pub", &key_dir, &selected_name);
        if Path::new(&key_path).exists() || Path::new(&pub_key_path).exists() {
            if delete_all_keys == "" {
                let delete_key_confirmation = read_from_input(&format!("Delete key for {selected_name} [ya/y/n/na]?"), None, vec!["ya", "y", "na"])?;
                if delete_key_confirmation == "ya" || delete_key_confirmation == "na" {
                    delete_all_keys = delete_key_confirmation.clone();
                }
                
                // store for deletion
                if delete_key_confirmation == "y" || delete_key_confirmation == "ya" {
                    to_be_deleted_keys.extend([key_path.clone(), pub_key_path.clone()]);
                }
            } else if delete_all_keys == "ya" {
                to_be_deleted_keys.extend([key_path.clone(), pub_key_path.clone()]);
            }

        }
        
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

    // delete keys
    for key in to_be_deleted_keys {
        if Path::new(&key).exists() {
            let _ = fs::remove_file(&key);
        }
    }
    
    Ok(())
}

async fn connect()-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;

    // start selection
    let selected = select_nodes().await?;

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
            let name: String = row.get("name");
            let address: String = row.get("address");
            let auth_type: String = row.get("auth_type");
            let default_key: bool = row.get("default_key");
            
            // execute ssh
            connect_to_server(&name, &address, &auth_type, default_key, &vec![])?;
        },
        "server container" => {
            // get from database
            let row = sqlx::query("SELECT 
                    nodes.*,
                    server.name as server_name,
                    server.address as server_address,
                    server.auth_type as server_auth_type
                    FROM nodes
                    JOIN nodes AS server ON nodes.parent_id = server.id
                    WHERE nodes.name = ?
                ")
                .bind(&selected)
                .fetch_one(&pool)
                .await?;
            
            // split address and port
            let container_address: String = row.get("address");
            let server_name: String = row.get("server_name");
            let server_address: String = row.get("server_address");
            let server_auth_type: String = row.get("server_auth_type");
            let server_default_key: bool = row.get("server_default_key");
            
            // execute ssh
            connect_to_server_container(&container_address, &server_name, &server_address, &server_auth_type, server_default_key, vec!["-t"])?;
        },
        _ => ()
    }

    Ok(())
}

async fn scan_server_container(scan_all: bool)-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;
    let rows;

    // select nodes if scan all is false
    if !scan_all {
        // start multi selection
        let selections = select_multi_server().await?;

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
        let auth_type: String = row.get("auth_type");
        let address: String = row.get("address");
        let default_key: bool = row.get("default_key");

        println!("Scanning container on : {name}");
        
        // ssh args
        let mut ssh_args = connect_to_server_args(&name, &address, &auth_type, default_key, &vec![])?;
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
                        auth_type,
                        parent_id
                    ) VALUES (
                        ?, 
                        ?,
                        ?,
                        ?,
                        ?
                    )"
                )
                .bind(&node_name)
                .bind(&container)
                .bind("server_container")
                .bind("container")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;

    Ok(())
}

async fn show_key()-> Result<(), Box<dyn std::error::Error>> {
    // start selection
    let selected = select_server().await?;

    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }

    println!("Selected: {selected}");
    let (_prefix, selected) = split_selected(&selected);

    let key_dir = key_dir()?;
    let key_path = format!("{}/{}.pub", &key_dir, &selected);
    if Path::new(&key_path).exists() {
        let key_content = fs::read_to_string(&key_path)?;
        println!("{key_content}");
    } else {
        eprintln!("Key not found for: {selected}");
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
    } else if args.delete {
        delete_node().await?;
    } else if args.scan_all_server_container {
        scan_server_container(true).await?;
    } else if args.scan_server_container {
        scan_server_container(false).await?;
    } else if args.key {
        show_key().await?;
    }

    Ok(())
}