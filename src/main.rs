pub mod helper;
pub mod migration;
pub mod database;
pub mod config;

use clap::Parser;
use std::env;
use std::process::{Command, Stdio};
use crate::helper::{read_from_input, split_selected};
use crate::config::{tmp_list_file};
use crate::migration::migrate;
use crate::database::get_db_pool;
use std::io::{Write};
use std::fs::File;
use sqlx::{Row};
use crate::config::{init_config};


#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    new: bool,
    #[arg(short, long)]
    delete: bool,
    #[arg(short, long)]
    edit: bool,
}

async fn add_node() -> Result<(), Box<dyn std::error::Error>> {
    let name = read_from_input("Name", None, vec![])?;
    let user = read_from_input("User", None, vec![])?;
    let host = read_from_input("Host", None, vec![])?;
    let port = read_from_input("Port (22)", Some("22"), vec![])?;
    let auth_type_choice = read_from_input("Auth Type [(k)ey/(p)assword]?", None, vec!["k", "p"])?;

    let mut auth_type = "key";
    match auth_type_choice.as_str() {
        // key
        "k" => {
            let generate_key = &read_from_input("Generate new key (use default key if no) [y/n]?", None, vec!["y", "n"])?;
        },
        // password
        "p" => {
            auth_type = "password";
        }
        _ => {}
    }

    let port = if port == "" { "22" } else { &port };
    let address = format!("{user}@{host}:{port}");

    let pool = get_db_pool().await?;
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
        .execute(&pool)
        .await?;

    Ok(())
}

async fn delete_node() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;

    let pool = get_db_pool().await?;
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

    // format raw selected
    let mut selected_names = Vec::new();
    for selected in selections {
        let (prefix, selected) = split_selected(&selected);
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

    let mut q = sqlx::query(&query);
    for selected_name in &selected_names {
        q = q.bind(selected_name);
    }
    q.execute(&pool).await?;
    
    Ok(())
}

async fn connect()-> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;

    // get local active container
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}"])
        .output()
        .expect("failed to run docker");

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
        writeln!(file, "server: {name}")?;
    }

    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
        .stdin(Stdio::from(file))
        .output()?;

    let selected = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    // get selected
    println!("Selected: {selected}");
    let (prefix, selected) = split_selected(&selected);

    // connect
    match prefix.as_str() {
        "container" => {
            Command::new("docker")
                .args(["exec", "-it", &selected, "sh"])
                .status()?;
        },
        "server" => {
            let row = sqlx::query("SELECT * FROM nodes WHERE name = ?")
                .bind(&selected)
                .fetch_one(&pool)
                .await?;
            
            let address: String = row.get("address");
            let address: Vec<&str> = address.split(":").collect();
            
            Command::new("ssh")
                .args([&address[0], "-p", &address[1]])
                .status()?;
        },
        _ => ()
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
        let _ = connect().await;
    }

    // get args
    let args = Args::parse();

    if args.new {
        add_node().await?;
    }
    
    if args.delete {
        delete_node().await?;
    }

    Ok(())
}