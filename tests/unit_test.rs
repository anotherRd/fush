use std::{assert_eq, assert_ne, format, fs::{self, OpenOptions}};

use fush::{config::{key_dir}, helper::{create_key_pair, key_pair_exists}};
use tokio::sync::OnceCell;

// create key pair if file not exists
// create key pair is not complete
// create key pair if exists but overwrite is true
// dont create key pair if overwrite is false

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
    let key_dir = key_dir().unwrap();
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&format!("{key_dir}/{name}")).unwrap();
}

fn read_file_dummy(name: &str) -> String {
    let key_dir = key_dir().unwrap();
    let key_path = format!("{key_dir}/{name}");
    let key_content = fs::read_to_string(&key_path).unwrap();
    key_content
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