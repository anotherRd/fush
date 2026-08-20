use std::io::{self, Write};
use crate::config::{key_dir, tmp_list_file};
use crate::debug_println;
use std::path::Path;
use std::process::{Command, Stdio};
use std::fs::File;
use crate::database::get_db_pool;
use sqlx::Row;
use std::fs;

pub fn check_requirement() {
    // optional requirement
    let optional = vec![
        "docker",
    ];

    let optinal: Vec<_> = optional
        .into_iter()
        .filter(|command| which::which(command).is_err())
        .collect();

    for command in &optinal {
        custom_print("warning", &format!("missing: {command})"));
    }

    // mandatory requirement
    let mandatory = vec![
        "ssh",
        "ssh-keygen",
        "fzf",
    ];

    let mandatory: Vec<_> = mandatory
        .into_iter()
        .filter(|command| which::which(command).is_err())
        .collect();

    for command in &mandatory {
        custom_print("error", &format!("missing: {command})"));
    }

    if mandatory.len() > 0 {
        std::process::exit(1);
    }
}

pub fn read_from_input(
    caption: &str,
    default: Option<&str>,
    choices: Vec<&str>,
    required: bool
) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let mut trimmed_input = "";
    let mut continue_loop = true;

    while continue_loop {
        continue_loop = false;
        // read from input
        print!("{caption}: ");
        io::stdout().flush()?;
        input.clear();
        io::stdin().read_line(&mut input)?;
        trimmed_input = input.trim();

        // if empty
        if trimmed_input == "" {
            // set to default if available
            if let Some(default_value) = default {
                trimmed_input = default_value;
            } else if required{
                custom_print("warning", &format!("{caption} is required"));
                continue_loop = true;
            }
        } else if choices.len() > 0 {
            // if choices available
            if !choices.contains(&trimmed_input) {
                let choice_values = choices.join("/");
                custom_print("warning", &format!("{caption} value must be [{choice_values}]"));
                continue_loop = true;
            }
        }
    }

    Ok(trimmed_input.to_string())
}

pub fn split_selected(selected: &str) -> (String, String) {
    let prefix: Vec<&str> = selected.split(": ").collect();
    let selected = selected.replacen(&format!("{}: ", &prefix[0]), "", 1);

    (prefix[0].to_string(), selected)
}

pub fn connect_to_server_args<'a>(
    key: &'a Option<&'a str>,
    address: &'a str,
    additional_args: &Vec<&'a str>
) -> Result<Vec<String>, Box<dyn std::error::Error>> { 
    // split address and port
    let address: Vec<&str> = address.split(":").collect();

    // ssh args
    let mut ssh_args = vec![
        "-o".to_string(),
        "ConnectTimeout=5".to_string(),
        address[0].to_string(),
        "-p".to_string(),
        address[1].to_string()
    ];

    // check if using key or password
    if let Some(key) = &key {
        if key_pair_exists(&key)? {
            let key_dir = key_dir()?;
            let key_path = format!("{}/{}", &key_dir, &key);
            ssh_args.extend(["-i".to_string(), key_path.to_string()]);
        }
    }

    for additional_arg in additional_args {
        ssh_args.push(additional_arg.to_string());
    }

    Ok(ssh_args)
}

pub fn connect_to_server(
    key: &Option<&str>,
    address: &str,
    additional_args: &Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> { 
    // prepare command
    let ssh_args = connect_to_server_args(&key, &address, &additional_args)?;
    let mut cmd = Command::new("ssh");
    cmd.args(&ssh_args);
    
    // print before executed
    debug_println!("{:?}", cmd);

    // execute
    cmd.status()?;
    
    Ok(())
}

pub fn connect_to_container(
    address: &str,
    additional_args: Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> {
    let mut check_args = vec!["exec", &address, "sh", "-c"];
    check_args.extend(&additional_args);

    let shells = vec!["bash", "ash", "sh"];
    for shell in shells {
        // prepare shell check command
        let mut tmp_args = check_args.clone();
        tmp_args.push(&shell);

        let mut cmd = Command::new("docker");
        cmd.args(tmp_args);
        
        // print before executed
        debug_println!("{:?}", cmd);

        // execute
        let shell_status = cmd.status()?;
        
        // prepare connection command
        let mut connection_cmd = Command::new("docker");
        connection_cmd.args(["exec", "-it", &address, &shell]);
        debug_println!("{:?}", connection_cmd);
        if shell_status.success() {
            connection_cmd.status()?;
            break;
        }
    }

    Ok(())
}

pub fn connect_to_server_container(
    container_address: &str,
    server_key: &Option<&str>,
    server_address: &str,
    additional_args: Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> {
    let ssh_args = connect_to_server_args(&server_key, &server_address, &additional_args)?;
    let docker_command = format!("docker exec -it {container_address}");
    let docker_check_shell_command = format!("docker exec {container_address} sh -c");

    let shells = vec!["bash", "ash", "sh"];
    for shell in shells {
        // let tmp_docker_command = format!("{docker_command} {shell}");
        // let mut tmp_ssh_args = ssh_args.clone();
        // tmp_ssh_args.push(tmp_docker_command);
        
        // let shell_status = Command::new("ssh")
        //     .args(tmp_ssh_args)
        //     .status()?;
        
        // if shell_status.success() {
        //     break;
        // }
        
        // prepare shell check command
        let tmp_docker_command = format!("{docker_check_shell_command} {shell}");
        let mut tmp_ssh_args = ssh_args.clone();
        tmp_ssh_args.push(tmp_docker_command);
        
        let mut cmd = Command::new("ssh");
        cmd.args(&tmp_ssh_args);
        
        // print before executed
        debug_println!("{:?}", cmd);
        
        // execute
        let shell_status = cmd.status()?;
        
        // prepare connection command
        let tmp_docker_command = format!("{docker_command} {shell}");
        let mut tmp_ssh_args = ssh_args.clone();
        tmp_ssh_args.push(tmp_docker_command);
        
        let mut connection_cmd = Command::new("ssh");
        connection_cmd.arg("-t");
        connection_cmd.args(tmp_ssh_args);
        debug_println!("{:?}", connection_cmd);
        if shell_status.success() {
            connection_cmd.status()?;
            break;
        }
    }

    Ok(())
}


pub fn multi_selection(title: &str) -> Result <Vec<String>, Box<dyn std::error::Error>> { 
    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
        .arg("--multi")
        .arg(&format!("--header={title}"))
        .stdin(Stdio::from(file))
        .output()?;

    // get selection to vec
    let selections: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect();

    Ok(selections)
}

pub fn selection(title: &str) -> Result <String, Box<dyn std::error::Error>> { 
    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
        .arg(&format!("--header={title}"))
        .stdin(Stdio::from(file))
        .output()?;

    let selected = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    Ok(selected)
}

pub fn db_array_placeholders(data_length: usize) -> String {
    std::iter::repeat("?")
        .take(data_length)
        .collect::<Vec<_>>()
        .join(",")
}

pub async fn select_nodes(title: &str) -> Result <String, Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;

    // get local active container
    if let Ok(output) = Command::new("docker").args(["ps", "--format", "{{.Names}}"]).output() {
        // turn it to vec
        let containers: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();
    
        // write to tmp file
        for container in containers {
            writeln!(file, "container: {container}")?;
        }
    }

    // get nodes
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

    let selected = selection(&title)?;

    Ok(selected)
}

pub async fn select_server(title: &str) -> Result <String, Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;
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

    let selected = selection(&title)?;

    Ok(selected)
}

pub async fn select_multi_server(title: &str) -> Result <Vec<String>, Box<dyn std::error::Error>> {
    let mut file = File::create(&tmp_list_file())?;
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

    let selections = multi_selection(&title)?;

    Ok(selections)
}

pub fn key_pair_exists(key: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let key_dir = key_dir()?;
    let key_path = format!("{}/{}", &key_dir, &key);
    let pub_key_path = format!("{}/{}.pub", &key_dir, &key);
    return Ok(Path::new(&key_path).exists() && Path::new(&pub_key_path).exists());
}

pub fn create_key_pair(key: &str, overwrite: bool, default_passphrase: Option<String>) -> Result<bool, Box<dyn std::error::Error>> {
    let key_dir = key_dir()?;
    let key_path = format!("{}/{}", &key_dir, &key);
    let pub_key_path = format!("{}/{}.pub", &key_dir, &key);

    // if key pair exists and overwrite false return false
    if key_pair_exists(&key)? && !overwrite {
        return Ok(false);
    }

    // delete existing key if overwrite is true
    if Path::new(&key_path).exists() {
        fs::remove_file(&key_path)?;
    }
    
    if Path::new(&pub_key_path).exists() {
        fs::remove_file(&pub_key_path)?;
    }

    let key_location = format!("{}/{}", &key_dir, &key);
    let mut keygen_args = vec!["-f", &key_location];
    let passphare;
    if let Some(default_passphrase_value) = default_passphrase {
        passphare = default_passphrase_value.clone();
        keygen_args.extend(["-N", &passphare]);
    }
    Command::new("ssh-keygen")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .args(keygen_args)
        .output()?;

    return Ok(true);

}

pub fn split_server_address(address: &str) -> (String, String, String) {
    // split address
    let (user, rest) = address.split_once('@').unwrap();
    let (host, port) = rest.split_once(':').unwrap();

    (user.to_string(), host.to_string(), port.to_string())
}

pub fn custom_print(message_type: &str, message: &str) {
    match message_type {
        "info" => {
            println!("Info:     {message}");
        },
        "success" => {
            println!("Success:  {message}");
        },
        "warning" => {
            println!("Warning:  {message}");
        },
        "error" =>  {
            eprintln!("Error:   {message}");

        },
        _ => {
            println!("{message}");
        }
    }
}