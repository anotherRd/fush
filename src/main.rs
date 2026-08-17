pub mod helper;
pub mod migration;
pub mod database;
pub mod config;

use clap::{Parser, CommandFactory};
use std::env;
use std::process::{Command, Stdio};
use crate::helper::{read_from_input, split_selected};
use crate::config::{tmp_list_file, key_dir};
use crate::migration::migrate;
use crate::database::get_db_pool;
use std::io::{Write};
use std::fs::File;
use sqlx::{Row};
use crate::config::{init_config};
use std::path::Path;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    connect: bool,
    #[arg(short, long)]
    new: bool,
    #[arg(short, long)]
    delete: bool,
    #[arg(short, long)]
    edit: bool,
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

    // generate key for this node
    if generate_key == "y" {
        let key_location = format!("{}/{}", &key_dir()?, name);
        Command::new("ssh-keygen")
            .args(["-f", &key_location])
            .output()?;
    }

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

    // format selected raw 
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

    // bind actual data
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

    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }

    println!("Selected: {selected}");
    let (prefix, selected) = split_selected(&selected);

    // connect
    match prefix.as_str() {
        "container" => {
            let shells = vec!["bash", "ash", "sh"];
            for shell in shells {
                let shell_status = Command::new("docker").args(["exec", "-it", &selected, &shell]).status()?;
                if shell_status.success() {
                    break;
                }
            }
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
            let address: Vec<&str> = address.split(":").collect();

            // ssh args
            let mut ssh_args = vec![&address[0], "-p", &address[1]];

            // check if theres keys
            let key_dir = key_dir()?;
            let key_path = format!("{}/{}", &key_dir, &name);
            if Path::new(&key_path).exists() {
                ssh_args.extend(["-i", &key_path]);
            }
            
            // execute ssh
            Command::new("ssh")
                .args(ssh_args)
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
        Args::command().print_help()?;
    }

    // get args
    let args = Args::parse();

    if args.connect {
        connect().await?;
    }
    
    if args.new {
        new_node().await?;
    }
    
    if args.delete {
        delete_node().await?;
    }

    Ok(())
}