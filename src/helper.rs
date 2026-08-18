use std::io::{self, Write};
use crate::config::{key_dir, tmp_list_file};
use std::path::Path;
use std::process::{Command, Stdio};
use std::fs::File;

pub fn read_from_input(caption: &str, default: Option<&str>, choices: Vec<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let mut trimmed_input = "";

    while trimmed_input == "" {
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
            } else {
                println!("{caption} is required");
            }
        } else if choices.len() > 0 {
            // if choices available
            if !choices.contains(&trimmed_input) {
                let choice_values = choices.join("/");
                println!("{caption} value must be [{choice_values}]");
                trimmed_input = "";
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
    name: &'a str,
    address: &'a str,
    auth_type: &'a str,
    default_key: bool,
    additional_args: &Vec<&'a str>
) -> Result<Vec<String>, Box<dyn std::error::Error>> { 
    // split address and port
    let address: Vec<&str> = address.split(":").collect();

    // ssh args
    let mut ssh_args = vec![
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        address[0].to_string(),
        "-p".to_string(),
        address[1].to_string()
    ];

    // check if using key or password
    if auth_type == "key" && !default_key {
        // check key exists
        let key_dir = key_dir()?;
        let key_path = format!("{}/{}", &key_dir, &name);
        let pub_key_path = format!("{}/{}.pub", &key_dir, &name);
        if Path::new(&key_path).exists() && Path::new(&pub_key_path).exists() {
            ssh_args.extend(["-i".to_string(), key_path.to_string()]);
        }
    }

    for additional_arg in additional_args {
        ssh_args.push(additional_arg.to_string());
    }

    Ok(ssh_args)
}

pub fn connect_to_server(
    name: &str,
    address: &str,
    auth_type: &str,
    default_key: bool,
    additional_args: &Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> { 
    let ssh_args = connect_to_server_args(&name, &address, &auth_type, default_key, &additional_args)?;
    Command::new("ssh")
        .args(ssh_args)
        .status()?;
    
    Ok(())
}

pub fn connect_to_container(
    address: &str,
    additional_args: Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> { 
    let mut args = vec!["exec", "-it", &address];
    args.extend(additional_args);

    let shells = vec!["bash", "ash", "sh"];
    for shell in shells {
        let mut tmp_args = args.clone();
        tmp_args.push(&shell);

        let shell_status = Command::new("docker")
            .args(tmp_args)
            .status()?;
        
        if shell_status.success() {
            break;
        }
    }

    Ok(())
}

pub fn connect_to_server_container(
    container_address: &str,
    server_name: &str,
    server_address: &str,
    server_auth_type: &str,
    server_default_key: bool,
    additional_args: Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> {
    let ssh_args = connect_to_server_args(&server_name, &server_address, &server_auth_type, server_default_key, &additional_args)?;
    let docker_command = format!("docker exec -it {container_address}");

    let shells = vec!["bash", "ash", "sh"];
    for shell in shells {
        let tmp_docker_command = format!("{docker_command} {shell}");
        let mut tmp_ssh_args = ssh_args.clone();
        tmp_ssh_args.push(tmp_docker_command);
        
        let shell_status = Command::new("ssh")
            .args(tmp_ssh_args)
            .status()?;
        
        if shell_status.success() {
            break;
        }
    }

    Ok(())
}


pub fn multi_selection() -> Result <Vec<String>, Box<dyn std::error::Error>> { 
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

    Ok(selections)
}

pub fn selection() -> Result <String, Box<dyn std::error::Error>> { 
    // start fzf
    let file = File::open(&tmp_list_file())?;
    let output = Command::new("fzf")
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