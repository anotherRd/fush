use std::{assert_eq, assert_ne, fs::{self, OpenOptions}};

use fush::{config::key_dir, helper::{check_requirement, create_key_pair, key_pair_exists}};
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn setup() {
    SETUP
        .get_or_init(|| async {
            // create unit test dir
            let key_dir = key_dir().unwrap();
            for entry in fs::read_dir(&key_dir).unwrap() {
                let path = entry.unwrap().path();

                if path.is_file() {
                    fs::remove_file(path).unwrap();
                }
            }
        })
        .await;
}

fn create_file_dummy(name: &str) {
    let key_dir = key_dir().unwrap().join(name);
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(key_dir).unwrap();
}

fn read_file_dummy(name: &str) -> String {
    let key_dir = key_dir().unwrap();
    let key_path = key_dir.join(name);
    let key_content = fs::read_to_string(&key_path).unwrap();
    key_content
}

#[tokio::test]
async fn test_requirement_installed() {
    setup().await;

    let check_requirement = check_requirement();
    assert!(check_requirement.is_ok());
}

#[tokio::test]
async fn test_key_pair_exists() {
    setup().await;

    // key not found = false
    assert_eq!(false, key_pair_exists("unit_check_non_existing_key1").unwrap());
    
    // private key only = false
    create_file_dummy("unit_check_private_only");
    assert_eq!(false, key_pair_exists("unit_check_private_only").unwrap());
    
    // public key only = false
    create_file_dummy("unit_check_public_only.pub");
    assert_eq!(false, key_pair_exists("unit_check_public_only").unwrap());
    
    // both exists = true
    create_file_dummy("unit_check_complete");
    create_file_dummy("unit_check_complete.pub");
    assert_eq!(true, key_pair_exists("unit_check_complete").unwrap());
}

#[tokio::test]
async fn test_create_key_pair_exists() {
    setup().await;

    // file not existed
    assert_eq!(true, create_key_pair("unit_not_existed", false, Some(String::new())).unwrap());
    assert_eq!(true, key_pair_exists("unit_not_existed").unwrap());
    
    // file content stay the same if overwrite false
    let file1 = "unit_complete";
    let file2 = "unit_complete.pub";
    
    create_file_dummy(&file1);
    create_file_dummy(&file2);
    
    let file1_content_before = read_file_dummy(&file1);
    let file2_content_before = read_file_dummy(&file2);

    assert_eq!(false, create_key_pair("unit_complete", false, None).unwrap());

    let file1_content_after = read_file_dummy(&file1);
    let file2_content_after = read_file_dummy(&file2);

    assert_eq!(file1_content_before, file1_content_after);
    assert_eq!(file2_content_before, file2_content_after);

    // file content change if overwrite false
    let file1 = "unit_complete_overwrite";
    let file2 = "unit_complete_overwrite.pub";
    
    create_file_dummy(&file1);
    create_file_dummy(&file2);
    
    let file1_content_before = read_file_dummy(&file1);
    let file2_content_before = read_file_dummy(&file2);

    assert_eq!(true, create_key_pair("unit_complete_overwrite", true, Some(String::new())).unwrap());

    let file1_content_after = read_file_dummy(&file1);
    let file2_content_after = read_file_dummy(&file2);

    assert_ne!(file1_content_before, file1_content_after);
    assert_ne!(file2_content_before, file2_content_after);
    
    // file content change if only one file exists regardless of the overwite status
    // private only
    let file1 = "unit_private_only_overwrite";
    
    create_file_dummy(&file1);
    
    let file1_content_before = read_file_dummy(&file1);

    assert_eq!(true, create_key_pair("unit_private_only_overwrite", false, Some(String::new())).unwrap());
    assert_eq!(true, key_pair_exists("unit_private_only_overwrite").unwrap());

    let file1_content_after = read_file_dummy(&file1);

    assert_ne!(file1_content_before, file1_content_after);
    
    // public only
    let file1 = "unit_public_only_overwrite.pub";
    
    create_file_dummy(&file1);
    
    let file1_content_before = read_file_dummy(&file1);

    assert_eq!(true, create_key_pair("unit_public_only_overwrite", false, Some(String::new())).unwrap());
    assert_eq!(true, key_pair_exists("unit_public_only_overwrite").unwrap());

    let file1_content_after = read_file_dummy(&file1);

    assert_ne!(file1_content_before, file1_content_after);
}

#[tokio::test]
async fn test_blacklisted_key_name() {
    setup().await;

    assert_eq!(true, create_key_pair("config", false, Some(String::new())).is_err());
    assert_eq!(true, create_key_pair("known_hosts", false, Some(String::new())).is_err());
    assert_eq!(true, create_key_pair("known_hosts", false, Some(String::new())).is_err());
    assert_eq!(true, create_key_pair("authorized_keys", false, Some(String::new())).is_err());
    assert_eq!(true, create_key_pair("authorized_keys2", false, Some(String::new())).is_err());
    assert_eq!(true, create_key_pair("environment", false, Some(String::new())).is_err());
    assert_eq!(true, create_key_pair("rc", false, Some(String::new())).is_err());
}