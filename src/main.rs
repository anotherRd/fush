pub mod helper;
pub mod migration;
pub mod database;
pub mod config;

use clap::{Parser, CommandFactory};
use std::env;
use std::process::{Command, Stdio};
use crate::helper::{read_from_input, split_selected, connect_to_server, connect_to_container, connect_to_server_container};
use crate::config::{tmp_list_file, key_dir};
use crate::migration::migrate;
use crate::database::get_db_pool;
use std::io::{Write};
use std::fs::File;
use sqlx::{Row};
use crate::config::{init_config};
use std::path::Path;
use std::fs;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    connect: bool,
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
    match auth_type_choice.as_str() {
        // key
        "k" => {
            auth_type = "key";
            generate_key = read_from_input("Generate new key (use default key if no) [y/n]?", None, vec!["y", "n"])?;
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
                auth_type
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
        .bind(&auth_type)
        .execute(&mut *tx)
        .await?;

    // generate key for this node
    if generate_key == "y" {
        let key_location = format!("{}/{}", &key_dir()?, name);
        Command::new("ssh-keygen")
            .args(["-f", &key_location])
            .output()?;
    }

    tx.commit().await?;

    Ok(())
}

async fn delete_node() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;

    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    let rows = sqlx::query("SELECT * FROM nodes WHERE node_type = 'server' or node_type = 'server_container'")
        .fetch_all(&pool)
        .await?;

    for row in rows {
        let name: String = row.get("name");
        writeln!(file, "server: {name}")?;
    }

    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
        .arg("--multi")
        .stdin(Stdio::from(file))
        .output()?;

    // get selection to vec
    let selections: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect();

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
        let (_prefix, selected) = split_selected(&selected);
        selected_names.push(selected);
    }

    // delete from db
    let placeholders = std::iter::repeat("?")
        .take(selected_names.len())
        .collect::<Vec<_>>()
        .join(",");

    let query = format!(
        "DELETE FROM nodes WHERE name IN ({})",
        placeholders
    );

    // bind actual data
    let mut q = sqlx::query(&query);
    for selected_name in &selected_names {
        q = q.bind(selected_name);
    }
    q.execute(&mut *tx).await?;

    tx.commit().await?;
    
    Ok(())
}

async fn connect()-> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;

    // get local active container
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}"])
        .output()?;

    // turn it to vec
    let containers: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect();

    // write to tmp file
    for container in containers {
        writeln!(file, "container: {container}")?;
    }

    // get from db
    let pool = get_db_pool().await?;
    let rows = sqlx::query("SELECT * FROM nodes")
        .fetch_all(&pool)
        .await?;

    // write to tmp file
    for row in rows {
        let name: String = row.get("name");
        let node_type: String = row.get("node_type");
        let node_type_caption = node_type.replace("_", " ");
        writeln!(file, "{node_type_caption}: {name}")?;
    }

    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
        .stdin(Stdio::from(file))
        .output()?;

    let selected = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

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
            
            // execute ssh
            connect_to_server(&name, &address, &vec![])?;
        },
        "server container" => {
            // get from database
            let row = sqlx::query("SELECT 
                    nodes.*,
                    server.name as server_name,
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
            let server_name: String = row.get("server_name");
            let server_address: String = row.get("server_address");
            
            // execute ssh
            connect_to_server_container(&container_address, &server_name, &server_address, vec!["-t"])?;
        },
        _ => ()
    }

    Ok(())
}

async fn scan_server_container()-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;

    let rows = sqlx::query("SELECT * FROM nodes WHERE node_type = 'server'")
        .fetch_all(&pool)
        .await?;

    // write to tmp file
    for row in rows {
        let id: i64 = row.get("id");
        let name: String = row.get("name");
        let address: String = row.get("address");
        let address: Vec<&str> = address.split(":").collect();
        
        // ssh args
        let mut ssh_args = vec![&address[0], "-p", &address[1]];

        // check if theres keys
        let key_dir = key_dir()?;
        let key_path = format!("{}/{}", &key_dir, &name);
        if Path::new(&key_path).exists() {
            ssh_args.extend(["-i", &key_path]);
        }
        ssh_args.extend(["-t", "docker ps --format {{.Names}}"]);

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
    let mut file = File::create(&tmp_list_file())?;

    // get from db
    let pool = get_db_pool().await?;
    let rows = sqlx::query("SELECT * FROM nodes WHERE node_type = 'server'")
        .fetch_all(&pool)
        .await?;

    // write to tmp file
    for row in rows {
        let name: String = row.get("name");
        let node_type: String = row.get("node_type");
        let node_type_caption = node_type.replace("_", " ");
        writeln!(file, "{node_type_caption}: {name}")?;
    }

    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
        .stdin(Stdio::from(file))
        .output()?;

    let selected = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

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
        println!("Key not found for: {selected}");
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
    } else if args.scan_server_container {
        scan_server_container().await?;
    } else if args.key {
        show_key().await?;
    }

    Ok(())
}