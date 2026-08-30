use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::{fs};

pub fn is_test() -> bool {
    std::env::var_os("FUSH_TEST").is_some()
}

pub fn app_name() -> String {
    "fush".to_string()
}

pub fn config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // get config dir
    let config_dir;
    if is_test() {
        let dir = std::env::temp_dir().join(format!("{}_test", &app_name()));
        config_dir = dir;
    } else {
        let dir = dirs::config_dir()
            .ok_or("Config directory not found")?
            .join(format!("{}", &app_name()));
        config_dir = dir;
    }
    
    Ok(config_dir)
}

pub fn db_file() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = config_dir()?.join("data.db");
    return Ok(path);
}

pub fn key_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path;
    if is_test() {
        path = config_dir()?.join("keys");
    } else {
        path = dirs::home_dir()
            .ok_or("Home directory not found")?
            .join(".ssh");
    }
    return Ok(path);
}

pub fn init_config() -> Result<(), Box<dyn std::error::Error>> {
    // get config dir
    let config_dir = config_dir()?;

    // create config dir if not exists
    let path = Path::new(&config_dir);
    fs::create_dir_all(path)?;
    
    // create db file if not exists
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&db_file()?)?;

    // create key dir if not exists
    let key_dir = key_dir()?;
    let path = Path::new(&key_dir);
    fs::create_dir_all(path)?;

    Ok(())
}

pub fn get_requirements() -> (Vec<String>, Vec<String>) {
    let mandatory = vec![
        "sqlite3".to_string(),
        "ssh".to_string(),
        "ssh-keygen".to_string(),
    ];

    let optional = vec![
        "docker".to_string(),
    ];

    (mandatory, optional)
}