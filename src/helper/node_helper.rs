use std::{fs, path::Path, process::Command};

use sqlx::Row;

use crate::{config::{blacklisted_key_name, key_dir}, database::get_db_pool, debug_println, dto::node_dto::NodeDto, helper::skim_helper::{multi_selection, selection}};

pub fn get_wsl_list() -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let mut result = vec![];
    let output = Command::new("wsl")
        .args(["--list", "--verbose"])
        .output()?;

    let stdout = String::from_utf16_lossy(
        &output
            .stdout
            .chunks_exact(2)
            .map(|x| u16::from_le_bytes([x[0], x[1]]))
            .collect::<Vec<_>>(),
    );

    for line in stdout.lines().skip(1) {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let line = line.strip_prefix('*').unwrap_or(line).trim();

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let wsl_version = parts.last().unwrap();
            let state = parts[parts.len() - 2];
            let distro = parts[..parts.len() - 2].join(" ");

            result.push(vec![distro, state.to_string(), wsl_version.to_string()]);
        }
    }

    Ok(result)
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
            let key_dir = key_dir()?.join(key);
            let key_path = format!("{}", key_dir.display());
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

        // prepare connection command
        let mut connection_cmd = Command::new("docker");
        connection_cmd.args(["exec", "-it", &address, &shell]);
        debug_println!("{:?}", connection_cmd);

        // execute check docker shell
        let shell_status = cmd.status()?;
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
        
        if cfg!(target_os = "linux") {
            let mut connection_cmd = Command::new("ssh");
            connection_cmd.arg("-t");
            connection_cmd.args(tmp_ssh_args);
            debug_println!("{:?}", connection_cmd);
            if shell_status.success() {
                connection_cmd.status()?;
                break;
            }
        } else if cfg!(target_os = "windows") {
            let mut connection_cmd = Command::new("cmd.exe");
            connection_cmd.args([
                "/c",
                "start",
                "powershell.exe",
                "-NoExit",
                "-Command",
            ]);
            connection_cmd.arg(&format!("ssh -t {}", tmp_ssh_args.join(" ")));
            debug_println!("{:?}", connection_cmd);
            if shell_status.success() {
                connection_cmd.spawn()?;
                break;
            }
        }
    }

    Ok(())
}


pub fn connect_to_wsl_args<'a>(
    address: &'a str,
    additional_args: &Vec<&'a str>
) -> Result<Vec<&'a str>, Box<dyn std::error::Error>> { 
    let mut wsl_args = vec![
        "-d",
        address,
    ];
    wsl_args.extend(additional_args);

    Ok(wsl_args)
}

pub fn connect_to_wsl(
    address: &str,
    additional_args: &Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> { 
    // prepare command
    let wsl_args = connect_to_wsl_args(&address, &additional_args)?;
    let mut cmd = Command::new("wsl");
    cmd.args(&wsl_args);
    
    // print before executed
    debug_println!("{:?}", cmd);

    // execute
    cmd.status()?;
    
    Ok(())
}

pub fn connect_to_wsl_container(
    address: &str,
    wsl_address: &str,
    additional_args: &Vec<&str>
) -> Result<(), Box<dyn std::error::Error>> { 
    // prepare command
    let mut wsl_args = connect_to_wsl_args(&wsl_address, &additional_args)?;
    wsl_args.extend(["--", "sh", "-c"]);
    
    // check available container shell
    let docker_command = format!("docker exec -it {address}");
    let docker_check_shell_command = format!("docker exec {address} sh -c");

    
    let shells = vec!["bash", "ash", "sh"];
    for shell in shells {
        let tmp_docker_command = format!("{docker_check_shell_command} {shell}");
        
        let mut tmp_wsl_args = wsl_args.clone();
        tmp_wsl_args.push(&tmp_docker_command);

        let mut cmd = Command::new("wsl");
        cmd.args(&tmp_wsl_args);

        // print before executed
        debug_println!("{:?}", cmd);
        
        // execute
        let tmp_wsl_args = wsl_args.clone();
        let mut connection_cmd = Command::new("wsl");
        connection_cmd.args(tmp_wsl_args);
        connection_cmd.arg(&format!("{docker_command} {shell}"));
        debug_println!("{:?}", connection_cmd);

        let shell_status = cmd.status()?;
        if shell_status.success() {
            connection_cmd.status()?;
            break;
        }
    }
    
    Ok(())
}

pub fn print_server_container_detail(node_dto: &NodeDto) -> Result<(), Box<dyn std::error::Error>> {
    // docker command
    let format = concat!("CONTAINER:\n",
        "    Name: {{.Names}}\n",
        "    ID: {{.ID}}\n",
        "    Image: {{.Image}}\n",
        "    Status: {{.Status}}\n",
        "    Ports: {{.Ports}}",
    );
    let docker_command_arg = &format!(r#"docker ps -f name='^{}$' --format '{}'"#, &node_dto.address, &format);
    
    // parent
    if let Some(parent) = node_dto.parent.clone() {
        // server command args
        let key: Option<&str> = match &parent.key {
            Some(key) => Some(&key),
            None => None,
        };
        let ssh_args = connect_to_server_args(&key, &parent.address, &vec![&docker_command_arg])?;

        println!("SERVER:");
        println!("  Name: {}", parent.name);
        println!("  address: {}", parent.address);
        if let Some(key) = &parent.key {
            println!("  key: {}", key);
        } else {
            println!("  key: [default key]");
        }

        // prepare command
        let mut cmd = Command::new("ssh");
        cmd.args(ssh_args);
        
        // print before executed
        debug_println!("{:?}", cmd);

        // execute
        cmd.status()?;
    }

    Ok(())
}

pub fn print_wsl_detail(wsl_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let wsls = get_wsl_list()?;
    
    for wsl in wsls {
        if wsl[0] == wsl_name {
            println!("WSL:");
            println!("  Distro: {}", wsl[0]);
            println!("  State: {}", wsl[1]);
            println!("  WSL Version: {}", wsl[2]);
        }
    }

    Ok(())
}

pub fn print_wsl_container_detail(
    container: &str,
    wsl_name: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let wsls = get_wsl_list()?;
    
    for wsl in wsls {
        if wsl[0] == wsl_name {
            println!("WSL:");
            println!("  Distro: {}", wsl[0]);
            println!("  State: {}", wsl[1]);
            println!("  WSL Version: {}", wsl[2]);

            let format = concat!("CONTAINER:\n",
                "    Name: {{.Names}}\n",
                "    ID: {{.ID}}\n",
                "    Image: {{.Image}}\n",
                "    Status: {{.Status}}\n",
                "    Ports: {{.Ports}}",
            );
            let docker_command_arg = &format!(r#"docker ps -f name='^{}$' --format '{}'"#, &container, &format);
            
            let mut wsl_args = connect_to_wsl_args(&wsl_name, &vec![])?;
            wsl_args.extend(["--", "sh", "-c"]);

            let mut cmd = Command::new("wsl");
            cmd.args(wsl_args);
            cmd.arg(&docker_command_arg);

            cmd.status()?;

        }
    }

    Ok(())
}

pub fn print_server_detail(node_dto: &NodeDto) {
    println!("SERVER:");
    println!("  Name: {}", node_dto.name);
    println!("  Address: {}", node_dto.address);
    if let Some(key) = &node_dto.key {
        println!("  Key: {}", key);

    } else {
        println!("  Key: [default key]");
    }
}

pub fn print_container_detail(container_name: &str) -> Result<(), Box<dyn std::error::Error>>{
    let format = concat!("CONTAINER:\n",
        "  Name: {{.Names}}\n",
        "  ID: {{.ID}}\n",
        "  Image: {{.Image}}\n",
        "  Status: {{.Status}}\n",
        "  Ports: {{.Ports}}",
    );
    let mut cmd = Command::new("docker");
    cmd.args([
            "ps",
            "-f",
            &format!("name=^{container_name}$"),
            "--format",
            format,
        ]);

    // print before executed
    debug_println!("{:?}", cmd);
    
    let output = cmd.output()?;

    let result = String::from_utf8(output.stdout)?;

    println!("{result}");

    Ok(())
}

pub async fn select_server(title: &str) -> Result <String, Box<dyn std::error::Error>> {
    let mut candidate = String::new();
    let pool = get_db_pool().await?;
    let rows = sqlx::query("SELECT * FROM nodes WHERE node_type = 'server'")
        .fetch_all(&pool)
        .await?;

    // write to tmp file
    for row in rows {
        let name: String = row.get("name");
        let node_type: String = row.get("node_type");
        let node_type_caption = node_type.replace("_", " ");
        candidate.push_str(&format!("{node_type_caption}: {name}\n"));
    }

    let selected = selection(&title, candidate)?;

    Ok(selected)
}

pub async fn select_multi_server(title: &str) -> Result <Vec<String>, Box<dyn std::error::Error>> {
    let mut candidate = String::new();
    let pool = get_db_pool().await?;
    let rows = sqlx::query("SELECT * FROM nodes WHERE node_type = 'server'")
        .fetch_all(&pool)
        .await?;

    // write to tmp file
    for row in rows {
        let name: String = row.get("name");
        let node_type: String = row.get("node_type");
        let node_type_caption = node_type.replace("_", " ");
        candidate.push_str(&format!("{node_type_caption}: {name}\n"));
    }

    let selections = multi_selection(&title, candidate)?;

    Ok(selections)
}

pub async fn select_node(title: &str) -> Result <String, Box<dyn std::error::Error>> {
    let mut candidate = String::new();

    // get local active container
    if let Ok(output) = Command::new("docker").args(["ps", "--format", "{{.Names}}"]).output() {
        // turn it to vec
        let containers: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(String::from)
            .collect();
    
        // write to tmp file
        for container in containers {
            candidate.push_str(&format!("container: {container}\n"));
        }
    }
    
    // windows only
    if cfg!(target_os = "windows") {
        // wsl
        if let Ok(wsls) = get_wsl_list() {
            // write to tmp file
            for wsl in &wsls {
                candidate.push_str(&format!("wsl: {}\n", wsl[0]));
            }
            
            // get wsl container
            for wsl in &wsls {
                // if wsl is active write to file
                if wsl[1] == "Running" && let Ok(output_container) = Command::new("wsl").args(["-d", &wsl[0], "--", "sh", "-c", "docker ps --format {{.Names}}"]).output() {
                    let wsls_containers: Vec<String> = String::from_utf8_lossy(&output_container.stdout)
                        .lines()
                        .map(String::from)
                        .collect();

                    for container in &wsls_containers {
                        candidate.push_str(&format!("wsl container: {}: {container}\n", wsl[0]));
                    }
                }
            }
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
        candidate.push_str(&format!("{node_type_caption}: {name}\n"));
    }

    let selected = selection(title, candidate)?;

    Ok(selected)
}

pub fn key_pair_exists(key: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let key_dir = key_dir()?;
    let key_path = format!("{}", key_dir.join(&key).display());
    let pub_key_path = format!("{}.pub", key_dir.join(&key).display());
    return Ok(Path::new(&key_path).exists() && Path::new(&pub_key_path).exists());
}

pub fn create_key_pair(key: &str, overwrite: bool, default_passphrase: Option<String>) -> Result<bool, Box<dyn std::error::Error>> {
    let key_dir = key_dir()?;
    let key_path = format!("{}", key_dir.join(&key).display());
    let pub_key_path = format!("{}.pub", key_dir.join(&key).display());

    // check for not allowed names
    let blacklisted_names = blacklisted_key_name();

    if blacklisted_names.contains(&key) {
        return Err(format!("Error: key with name {key} is not allowed").into());
    }

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

    let key_location = format!("{}", key_dir.join(&key).display());
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