use std::{fs::{self, OpenOptions}, path::Path};

use crate::{config::{config_dir, db_file, key_dir}, helper::general_helper::check_requirement, migration::migrate};

pub fn dirs_and_files_setup() -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn main_setup() -> Result<(), Box<dyn std::error::Error>> {
    // check requirement
    check_requirement()?;

    // create required dirs and files
    dirs_and_files_setup()?;

    // migrate db
    migrate().await?;

    Ok(())
}