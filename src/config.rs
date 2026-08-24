use std::fs::OpenOptions;
use std::path::Path;
use std::{fs};

pub fn is_test() -> bool {
    std::env::var_os("FUSH_TEST").is_some()
}

pub fn app_name() -> String {
    "fush".to_string()
}

pub fn config_dir() -> Result<String, Box<dyn std::error::Error>> {
    // get config dir
    let config_dir;
    if is_test() {
        let dir = std::env::temp_dir().to_string_lossy().to_string();
        config_dir = format!("{}/{}_test", &dir, &app_name());
    } else {
        let dir = dirs::config_dir()
            .ok_or("Config directory not found")?
            .to_string_lossy()
            .to_string();
        config_dir = format!("{}/{}", &dir, &app_name());
    }
    

    Ok(config_dir)
}

pub fn tmp_dir() -> String {
    let tmp_dir = std::env::temp_dir().to_string_lossy().to_string();
    return format!("/{}/{}", &tmp_dir, &app_name());
}

pub fn db_file() -> Result<String, Box<dyn std::error::Error>> {
    return Ok(
        format!("{}/data.db", &config_dir()?)
    );
}

pub fn key_dir() -> Result<String, Box<dyn std::error::Error>> {
    return Ok(
        format!("{}/keys", &config_dir()?)
    );
}

pub fn init_config() -> Result<(), Box<dyn std::error::Error>> {
    // get config dir
    let config_dir = config_dir()?;

    // create config dir if not exists
    let path = Path::new(&config_dir);
    fs::create_dir_all(path)?;
    
    // create tmp dir if not exists
    let tmp_dir = tmp_dir();
    let tmp_dir_path = Path::new(&tmp_dir);
    fs::create_dir_all(tmp_dir_path)?;
    
    // create db file if not exists
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(&db_file()?)?;

    // create config dir if not exists
    let key_dir = key_dir()?;
    let path = Path::new(&key_dir);
    fs::create_dir_all(path)?;

    Ok(())
}

pub fn tmp_list_file() -> String {
    let tmp_dir = tmp_dir();
    return format!("{}/list", &tmp_dir);
}

pub fn get_requirements() -> (Vec<String>, Vec<String>) {
    let mandatory = vec![
        "sqlite3".to_string(),
        "ssh".to_string(),
        "ssh-keygen".to_string(),
        "fzf".to_string(),
    ];

    let optional = vec![
        "docker".to_string(),
    ];

    (mandatory, optional)
}