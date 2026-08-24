use clap::{CommandFactory, Parser};
use fush::custom_command::{Cli, Commands};
use fush::debug_println;
use std::{println, vec};
use std::process::{Command};
use fush::helper::{auto_complete_key, check_requirement, connect_to_container, connect_to_server, connect_to_server_args, connect_to_server_container, connect_to_wsl, connect_to_wsl_container, custom_print, db_array_placeholders, key_pair_exists, print_container_detail, print_server_container_detail, print_server_detail, read_from_input, select_multi_server, select_nodes, select_server, split_selected, split_server_address};
use fush::config::{is_test, key_dir};
use fush::migration::migrate;
use fush::database::get_db_pool;
use fush::service::node_service::{self, find_node_by_name};
use fush::service_params::node_service_params::{EditServerServiceParams, AddServerServiceParams};
use sqlx::{Row};
use fush::config::{init_config};
use std::fs;

async fn add_server() -> Result<(), Box<dyn std::error::Error>> {
    // read user input
    custom_print("info", "Add new server");
    let name = read_from_input("Name", None, vec![], true)?;
    let user = read_from_input("User", None, vec![], true)?;
    let host = read_from_input("Host", None, vec![], true)?;
    let port = read_from_input("Port (22)", Some("22"), vec![], true)?;
    let key = auto_complete_key("Key name (create if not exists and use default key/password if empty)", Some(""), vec![], false)?;
    
    // save new node
    node_service::add_server(AddServerServiceParams {
        name: name,
        user: user,
        host: host,
        port: port,
        key: key,
        default_passphrase: None
    }).await?;

    Ok(())
}

async fn edit_server(selected: String) -> Result<(), Box<dyn std::error::Error>> {
    if selected == "" {
        return Ok(());
    }
    
    // format selected
    let (_prefix, selected_name) = split_selected(&selected);

    // get server
    let node_dto = node_service::find_node_by_name(&selected_name).await?;
    let (old_user, old_host, old_port) = split_server_address(&node_dto.address);

    // read user input
    custom_print("info", &format!("Edit server: {selected_name}"));
    let name = read_from_input(&format!("Name ({})", &node_dto.name), Some(&node_dto.name), vec![], true)?;
    let user = read_from_input(&format!("User ({old_user})"), Some(&old_user), vec![], true)?;
    let host = read_from_input(&format!("Host ({old_host})"), Some(&old_host), vec![], true)?;
    let port = read_from_input(&format!("Port ({old_port})"), Some(&old_port), vec![], true)?;
    let change_key_caption = match &node_dto.key {
        Some(key) => format!("Change key ({}) [y/n]?", key),
        None => "Change key [y/n]?".to_string(),
    };
    let change_key = read_from_input(&change_key_caption, None, vec!["y", "n"], true)?;

    let mut key = node_dto.key.unwrap_or("".to_string());
    if change_key == "y" {
        key = auto_complete_key("Key name (create if not exists and use default key/password if empty)", Some(""), vec![], false)?;
    }

    // save edit node
    node_service::edit_server(node_dto.id, EditServerServiceParams {
        name: name,
        user: user,
        host: host,
        port: port,
        key: key,
        default_passphrase: None
    }).await?;

    Ok(())
}

async fn delete_server(selections: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // if nothing is selected
    if selections.is_empty() {
        return Ok(());
    }

    // confirmation
    custom_print("warning", &format!("To be deleted:"));
    println!("[{}]", &selections.join(", "));
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

    // delete server
    node_service::delete_server_by_names(&selected_names).await?;
    
    Ok(())
}

async fn connect(selected: String)-> Result<(), Box<dyn std::error::Error>> {
    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }
    custom_print("info", &format!("Selected: {selected}"));
    let (prefix, selected) = split_selected(&selected);

    // connect
    let pool = get_db_pool().await?;
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
            connect_to_server_container(&container_address, &server_key, &server_address, vec![])?;
        },
        "wsl" => {
            // execute docker exec
            connect_to_wsl(&selected, &vec![])?;
        },
        "wsl container" => {
            // execute docker exec
            let (wsl_address, container) = split_selected(&selected);
            connect_to_wsl_container(&container, &wsl_address, &vec![])?;
        },
        _ => ()
    }

    Ok(())
}

async fn scan_server_container(scan_all: bool, selections: Vec<String>, fake_containers: Vec<String>)-> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool().await?;
    let mut tx = pool.begin().await?;
    let rows;

    // select nodes if scan all is false
    if !scan_all {
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
        rows = q.fetch_all(&pool).await?;
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

        custom_print("info", &format!("scanning container on : {name}"));
        
        // ssh args
        let mut ssh_args = connect_to_server_args(&key, &address, &vec![])?;
        ssh_args.extend(["-t".to_string(), "docker ps --format {{.Names}}".to_string()]);

        // execute ssh
        let mut cmd = Command::new("ssh");
        cmd.args(ssh_args);
        debug_println!("{:?}", cmd);
        let output = cmd.output()?;

        // turn it to vec
        let mut containers: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();

        // temp: for test purpose
        if !fake_containers.is_empty() {
            containers = fake_containers.clone();
        }

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

    custom_print("info", &format!("finished"));

    Ok(())
}

async fn show_key(selected: String)-> Result<(), Box<dyn std::error::Error>> {
    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }

    custom_print("info", &format!("Selected: {selected}"));
    let (_prefix, selected_name) = split_selected(&selected);

    // get detail
    let node_dto = find_node_by_name(&selected_name).await?;
    if let Some(key_value) = &node_dto.key {
        if key_pair_exists(&key_value)? {
            let key_dir = key_dir()?.join(&key_value);
            let key_path = format!("{}.pub", key_dir.display());
            let key_content = fs::read_to_string(&key_path)?;
            println!("Key location: {key_path}\n");
            println!("{key_content}");
        } else {
            custom_print("error", &format!("Key {key_value} not found"));
        }
    } else {
        custom_print("info", &format!("Use default keys"));
    }


    Ok(())
}

async fn show_detail(selected: String)-> Result<(), Box<dyn std::error::Error>> {
    // return if nothing is selected
    if selected == "" {
        return Ok(())
    }

    let (prefix, selected_name) = split_selected(&selected);
    match prefix.as_str() {
        "container" => {
            print_container_detail(&selected_name)?;
        },
        "server" => {
            // get detail
            let node_dto = find_node_by_name(&selected_name).await?;
            print_server_detail(&node_dto);
        },
        "server container" => {
            let node_dto = find_node_by_name(&selected_name).await?;
            print_server_container_detail(&node_dto)?;
        },
        _ => ()
    }


    Ok(())
}

pub async fn prepare() -> Result<(), Box<dyn std::error::Error>> {
    // check requirement
    check_requirement()?;

    // init config
    init_config()?;

    // migrate db
    migrate().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    prepare().await?;

    // command
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Conenct { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_nodes("Select a node to connect").await?;
            }
            connect(selected).await?;
        },
        Some(Commands::Add) => {
            add_server().await?;
        },
        Some(Commands::Edit { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_server("Select a server to edit").await?;
            }
            edit_server(selected).await?;
        },
        Some(Commands::Delete { args }) => {
            let selections;
            if !args.is_empty() {
                selections = args;
            } else {
                selections = select_multi_server("Select server(s) to delete").await?;
            }
            delete_server(selections).await?;
        },
        Some(Commands::Scan { args, mut fake_container }) => {
            if !is_test() {
                fake_container = vec![];
            }

            let selections;
            if !args.is_empty() {
                selections = args;
            } else {
                selections = select_multi_server("Select server(s) to scan").await?;
            }
            scan_server_container(false, selections, fake_container).await?;
        },
        Some(Commands::ScanAll {mut fake_container}) => {
            if !is_test() {
                fake_container = vec![];
            }

            scan_server_container(true, vec![], fake_container).await?;
        },
        Some(Commands::ShowKey { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_server("Select a server to show the used key").await?;
            }
            show_key(selected).await?;
        }
        Some(Commands::ShowDetail { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_nodes("Select a node to show the detail").await?;
            }
            show_detail(selected).await?;
        },
        #[cfg(debug_assertions)]
        Some(Commands::Prepare) =>  {
            prepare().await?;
        }
        #[cfg(debug_assertions)]
        Some(Commands::Test) =>  {
            let status = Command::new("cargo")
                .env("FUSH_TEST", "1")
                .args(["run", "--", "prepare"])
                .status()?;

            if !status.success() {
                return Err(format!("{status}").into());
            }

            let status = Command::new("cargo")
                .env("FUSH_TEST", "1")
                .args(["test", "--", "--test-threads", "1"])
                .status()?;

            if !status.success() {
                return Err(format!("{status}").into());
            }
        }
        None => Cli::command().print_help()?
    }

    Ok(())
}