use fush::{helper::key_pair_exists, service::node_service::{self, delete_all, get_blacklisted_key_name}, service_params::node_service_params::AddServerServiceParams};
use rexpect::{error::Error, process::Signal, session::{PtySession, spawn_command}};
use sqlx::Row;
use tokio::sync::OnceCell;
use fush::database::get_db_pool;
use fush::config::key_dir;
use std::{assert_eq, format, fs, process::Command};

static SETUP: OnceCell<()> = OnceCell::const_new();
const BINARY: &str = env!("CARGO_BIN_EXE_fush");

async fn setup() {
    SETUP
        .get_or_init(|| async {
            let pool = get_db_pool().await.unwrap();
            let mut tx = pool.begin().await.unwrap();
            
            // delete all data
            sqlx::query("DELETE FROM nodes")
                .execute(&mut *tx)
                .await
                .unwrap();

            let key_dir = key_dir().unwrap();
            for entry in fs::read_dir(&key_dir).unwrap() {
                let path = entry.unwrap().path();

                if path.is_file() {
                    fs::remove_file(path).unwrap();
                }
            }
            tx.commit().await.unwrap();
        })
        .await;
}

fn spawn_test(args: &Vec<&str>, mut timeout_ms: Option<u64>) -> Result<rexpect::session::PtySession, Error> {
    if timeout_ms.is_none() {
        timeout_ms = Some(10_000);
    }
    let mut cmd = Command::new(BINARY);
    cmd.env("FUSH_TEST", "1");
    cmd.args(args);

    spawn_command(cmd, timeout_ms)
}

fn exp_string_assert(
    result: Result<String, rexpect::error::Error>,
    p: &mut PtySession,
    file: &str,
    line: u32,
    column: u32,
) {
    if let Err(error) = result {
        p.process_mut().kill(Signal::SIGKILL).unwrap();
        panic!("actual panicked at {file}:{line}:{column}\nExpected output was not found: {error:?}")
    } else {
        assert!(true);
    }
}

fn send_line_assert(
    result: Result<usize, rexpect::error::Error>,
    p: &mut PtySession,
    file: &str,
    line: u32,
    column: u32,
) {
    if let Err(error) = result {
        p.process_mut().kill(Signal::SIGKILL).unwrap();
        panic!("actual panicked at {file}:{line}:{column}\nSend line failed: {error:?}")
    } else {
        assert!(true);
    }
}

fn eof_assert(
    result: Result<String, rexpect::error::Error>,
    p: &mut PtySession,
    file: &str,
    line: u32,
    column: u32,
) {
    if let Err(error) = result {
        p.process_mut().kill(Signal::SIGKILL).unwrap();
        panic!("actual panicked at {file}:{line}:{column}\nEOF failed: {error:?}")
    } else {
        assert!(true);
    }
}

#[tokio::test]
async fn test_add_server() {
    setup().await;
    delete_all().await.unwrap();

    let name = "add_server";
    let user = "user_add_server";
    let host = "192.168.1.1";
    let port = "33";
    let key = "add_server_key";

    let mut p = spawn_test(&vec!["a"], None).unwrap();

    exp_string_assert(p.exp_string("Name: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("User: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&user), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Host: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&host), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Port (22): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&port), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Key name (create if not exists and use default key/password if empty): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&key), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Enter passphrase"), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(""), &mut p, file!(), line!(), column!());
    
    exp_string_assert(p.exp_string("Enter same passphrase again:"), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(""), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string("Success:"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check saved data
    let saved_data = node_service::find_node_by_name(&name).await.unwrap();
    assert_eq!(name, saved_data.name);
    assert_eq!(format!("{user}@{host}:{port}"), saved_data.address);
    assert_eq!(key, saved_data.key.unwrap());
    assert_eq!(None, saved_data.parent_id);

    // check key file
    assert!(key_pair_exists(key).unwrap());
}

#[tokio::test]
async fn test_add_server_default_value() {
    setup().await;
    delete_all().await.unwrap();

    let name = "add_server_default_value";
    let user = "user_add_server_default_value";
    let host = "192.168.1.1";
    let port = "";
    let key = "";

    let mut p = spawn_test(&vec!["a"], None).unwrap();

    exp_string_assert(p.exp_string("Name: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("User: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&user), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Host: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&host), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Port (22): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&port), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Key name (create if not exists and use default key/password if empty): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&key), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Success:"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check saved data
    let saved_data = node_service::find_node_by_name(&name).await.unwrap();
    assert_eq!(name, saved_data.name);
    assert_eq!(format!("{user}@{host}:22"), saved_data.address);
    assert_eq!(None, saved_data.key);
    assert_eq!(None, saved_data.parent_id);

    // check key file
    assert_eq!(false, key_pair_exists(key).unwrap());
}

#[tokio::test]
async fn test_add_server_duplicate_name() {
    setup().await;
    delete_all().await.unwrap();

    let name = "add_server_duplicate_name";
    let user = "user_add_server_duplicate_name";
    let host = "192.168.1.1";
    let port = "";
    let key = "";

    // create first server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // create node with same name
    let mut p = spawn_test(&vec!["a"], None).unwrap();

    exp_string_assert(p.exp_string("Name: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!("Warning: {name} is already exists")), &mut p, file!(), line!(), column!());
    
    exp_string_assert(p.exp_string("Name: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&format!("{name}2")), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("User: "), &mut p, file!(), line!(), column!());

    p.process_mut().kill(Signal::SIGKILL).unwrap();

    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_add_server_blacklisted_key() {
    setup().await;
    delete_all().await.unwrap();

    let name = "add_server_blacklisted_key";
    let user = "user_add_server_blacklisted_key";
    let host = "192.168.1.1";
    let port = "";
    let key = "authorized_keys";

    let mut p = spawn_test(&vec!["a"], None).unwrap();

    exp_string_assert(p.exp_string("Name: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("User: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&user), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Host: "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&host), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Port (22): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&port), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Key name (create if not exists and use default key/password if empty): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&key), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Key name can't be on of [{}]", &get_blacklisted_key_name().join(", "))), &mut p, file!(), line!(), column!());
    
    p.process_mut().kill(Signal::SIGKILL).unwrap();

    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_edit_server() {
    setup().await;
    delete_all().await.unwrap();

    let name_before = "edit_server_before";
    let user_before = "user_edit_server_before";
    let host_before = "192.168.1.1";
    let port_before = "11";
    let key_before = "";

    // create first server
    node_service::add_server(AddServerServiceParams {
        name: name_before.to_string(),
        user: user_before.to_string(),
        host: host_before.to_string(),
        port: port_before.to_string(),
        key: key_before.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // edit
    let name_after = "edit_server_after";
    let user_after = "user_edit_server_after";
    let host_after = "192.168.1.3";
    let port_after = "33";
    let key_after = "edit_server_key_after";

    let mut p = spawn_test(&vec!["e", &format!("server: {name_before}")], None).unwrap();

    exp_string_assert(p.exp_string(&format!("Name ({name_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("User ({user_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&user_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Host ({host_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&host_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Port ({port_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&port_after), &mut p, file!(), line!(), column!());
    
    if key_before == "" {
        exp_string_assert(p.exp_string(&format!("Change key [y/n]?: ")), &mut p, file!(), line!(), column!());
        send_line_assert(p.send_line("y"), &mut p, file!(), line!(), column!());
    } else {
        exp_string_assert(p.exp_string(&format!("Change key ({key_before}) [y/n]?: ")), &mut p, file!(), line!(), column!());
        send_line_assert(p.send_line("y"), &mut p, file!(), line!(), column!());
    }

    exp_string_assert(p.exp_string("Key name (create if not exists and use default key/password if empty): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&key_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Enter passphrase"), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(""), &mut p, file!(), line!(), column!());
    
    exp_string_assert(p.exp_string("Enter same passphrase again:"), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(""), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Success:"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check saved data
    let saved_data = node_service::find_node_by_name(&name_after).await.unwrap();
    assert_eq!(name_after, saved_data.name);
    assert_eq!(format!("{user_after}@{host_after}:{port_after}"), saved_data.address);
    assert_eq!(key_after, saved_data.key.unwrap());
    assert_eq!(None, saved_data.parent_id);

    // check key file
    assert!(key_pair_exists(key_after).unwrap());
}

#[tokio::test]
async fn test_edit_server_unchanged() {
    setup().await;
    delete_all().await.unwrap();

    let name_before = "edit_server_unchanged";
    let user_before = "user_edit_server_unchanged";
    let host_before = "192.168.1.1";
    let port_before = "11";
    let key_before = "";

    // create first server
    node_service::add_server(AddServerServiceParams {
        name: name_before.to_string(),
        user: user_before.to_string(),
        host: host_before.to_string(),
        port: port_before.to_string(),
        key: key_before.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // edit
    let name_after = "";
    let user_after = "";
    let host_after = "";
    let port_after = "";

    let mut p = spawn_test(&vec!["e", &format!("server: {name_before}")], None).unwrap();

    exp_string_assert(p.exp_string(&format!("Name ({name_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("User ({user_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&user_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Host ({host_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&host_after), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Port ({port_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&port_after), &mut p, file!(), line!(), column!());
    
    if key_before == "" {
        exp_string_assert(p.exp_string(&format!("Change key [y/n]?: ")), &mut p, file!(), line!(), column!());
        send_line_assert(p.send_line("n"), &mut p, file!(), line!(), column!());
    } else {
        exp_string_assert(p.exp_string(&format!("Change key ({key_before}) [y/n]?: ")), &mut p, file!(), line!(), column!());
        send_line_assert(p.send_line("n"), &mut p, file!(), line!(), column!());
    }

    exp_string_assert(p.exp_string("Success:"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check saved data
    let saved_data = node_service::find_node_by_name(&name_before).await.unwrap();
    assert_eq!(name_before, saved_data.name);
    assert_eq!(format!("{user_before}@{host_before}:{port_before}"), saved_data.address);
    assert_eq!(None, saved_data.key);
    assert_eq!(None, saved_data.parent_id);
}

#[tokio::test]
async fn test_edit_server_duplicate() {
    setup().await;
    delete_all().await.unwrap();

    let name_before = "edit_server_duplicate";
    let user_before = "user_edit_server_duplicate";
    let host_before = "192.168.1.1";
    let port_before = "11";
    let key_before = "";

    // create first server
    node_service::add_server(AddServerServiceParams {
        name: name_before.to_string(),
        user: user_before.to_string(),
        host: host_before.to_string(),
        port: port_before.to_string(),
        key: key_before.to_string(),
        default_passphrase: None
    }).await.unwrap();
    
    // create second server
    let name_second = "edit_server_duplicate2";
    let user_second = "user_edit_server_duplicate2";
    let host_second = "192.168.1.1";
    let port_second = "11";
    let key_second = "";
    node_service::add_server(AddServerServiceParams {
        name: name_second.to_string(),
        user: user_second.to_string(),
        host: host_second.to_string(),
        port: port_second.to_string(),
        key: key_second.to_string(),
        default_passphrase: None
    }).await.unwrap();

    let mut p = spawn_test(&vec!["e", &format!("server: {name_before}")], None).unwrap();

    exp_string_assert(p.exp_string(&format!("Name ({name_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name_second), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!("Warning: {name_second} is already exists")), &mut p, file!(), line!(), column!());
    
    exp_string_assert(p.exp_string(&format!("Name ({name_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&format!("{name_second}3")), &mut p, file!(), line!(), column!());
    
    exp_string_assert(p.exp_string(&format!("User ({user_before}): ")), &mut p, file!(), line!(), column!());

    p.process_mut().kill(Signal::SIGKILL).unwrap();
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_edit_server_blacklisted_key() {
    setup().await;
    delete_all().await.unwrap();

    let name_before = "edit_server_blacklisted_key";
    let user_before = "user_edit_server_blacklisted_key";
    let host_before = "192.168.1.1";
    let port_before = "11";
    let key_before = "";

    // create first server
    node_service::add_server(AddServerServiceParams {
        name: name_before.to_string(),
        user: user_before.to_string(),
        host: host_before.to_string(),
        port: port_before.to_string(),
        key: key_before.to_string(),
        default_passphrase: None
    }).await.unwrap();

    let mut p = spawn_test(&vec!["e", &format!("server: {name_before}")], None).unwrap();

    exp_string_assert(p.exp_string(&format!("Name ({name_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&name_before), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("User ({user_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&user_before), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Host ({host_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&host_before), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Port ({port_before}): ")), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line(&port_before), &mut p, file!(), line!(), column!());
    
    if key_before == "" {
        exp_string_assert(p.exp_string(&format!("Change key [y/n]?: ")), &mut p, file!(), line!(), column!());
        send_line_assert(p.send_line("y"), &mut p, file!(), line!(), column!());
    } else {
        exp_string_assert(p.exp_string(&format!("Change key ({key_before}) [y/n]?: ")), &mut p, file!(), line!(), column!());
        send_line_assert(p.send_line("y"), &mut p, file!(), line!(), column!());
    }

    exp_string_assert(p.exp_string("Key name (create if not exists and use default key/password if empty): "), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line("authorized_keys"), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string(&format!("Key name can't be on of [{}]", &get_blacklisted_key_name().join(", "))), &mut p, file!(), line!(), column!());

    p.process_mut().kill(Signal::SIGKILL).unwrap();
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_delete_server() {
    setup().await;
    delete_all().await.unwrap();

    let name = "delete_server";
    let user = "user_delete_server";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // delete
    let mut p = spawn_test(&vec!["d", &format!("server: {name}")], None).unwrap();

    exp_string_assert(p.exp_string("Are you sure [y/n]?"), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line("y"), &mut p, file!(), line!(), column!());

    exp_string_assert(p.exp_string("Success:"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check saved data
    let saved_data = node_service::get_node_by_names(&name).await.unwrap();
    assert!(saved_data.is_empty());
}

#[tokio::test]
async fn test_delete_server_cancel() {
    setup().await;
    delete_all().await.unwrap();

    let name = "delete_server_cancel";
    let user = "user_delete_server_cancel";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // delete
    let mut p = spawn_test(&vec!["d", &format!("server: {name}")], None).unwrap();

    exp_string_assert(p.exp_string("Are you sure [y/n]?"), &mut p, file!(), line!(), column!());
    send_line_assert(p.send_line("n"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check saved data
    let saved_data = node_service::get_node_by_names(&name).await.unwrap();
    assert_eq!(false, saved_data.is_empty());
}

#[tokio::test]
async fn test_scan_server_container_selecttion() {
    setup().await;
    delete_all().await.unwrap();

    let name = "scan_server_container_selecttion";
    let user = "user_scan_server_container_selecttion";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // scan
    let mut p = spawn_test(&vec!["s", &format!("server: {name}"), "-f", "fake-container-1", "-f", "fake-container-2"], None).unwrap();

    exp_string_assert(p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "-t" "docker ps --format {{{{.Names}}}}""#)), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string("finished"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check count
    let pool = get_db_pool().await.unwrap();
    let row = sqlx::query("SELECT COUNT(*) AS count FROM nodes WHERE address = 'fake-container-1' OR address = 'fake-container-2'")
        .fetch_one(&pool)
        .await.unwrap();
    let count: i64 = row.get("count");
    assert_eq!(2, count);
}

#[tokio::test]
async fn test_scan_server_container_all() {
    setup().await;
    delete_all().await.unwrap();

    let name = "scan_server_container_all";
    let user = "user_scan_server_container_all";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();


    // scan
    let mut p = spawn_test(&vec!["S", "-f", "fake-container-all-1", "-f", "fake-container-all-2"], None).unwrap();

    exp_string_assert(p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "-t""#)), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!(r#""docker ps --format {{{{.Names}}}}""#)), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string("finished"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // check count
    let pool = get_db_pool().await.unwrap();
    let row = sqlx::query("SELECT COUNT(*) AS count FROM nodes WHERE node_type = 'server_container' AND address != 'fake-container-all-1' AND address != 'fake-container-all-2'")
        .fetch_one(&pool)
        .await.unwrap();
    let count: i64 = row.get("count");
    assert_eq!(0, count);
}

#[tokio::test]
async fn test_connect_to_server() {
    setup().await;
    delete_all().await.unwrap();

    let name = "connect_to_server";
    let user = "user_connect_to_server";
    let host = "192.168.1.1";
    let port = "11";
    let key = "connect_to_server_key";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: Some("".to_string())
    }).await.unwrap();

    let key_dir = key_dir().unwrap().join(key);
    let key_path = format!("{}", key_dir.display());

    // connect
    let mut p = spawn_test(&vec!["c", &format!("server: {name}")], None).unwrap();

    exp_string_assert(p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "-i" "{key_path}""#)), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_connect_to_server_default_key() {
    setup().await;
    delete_all().await.unwrap();

    let name = "connect_to_server_default_key";
    let user = "user_connect_to_server_default_key";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();

    // connect
    let mut p = spawn_test(&vec!["c", &format!("server: {name}")], None).unwrap();

    exp_string_assert(p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}""#)), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_connect_to_container() {
    setup().await;
    delete_all().await.unwrap();

    let name = "connect_to_container";

    // conenct
    let mut p = spawn_test(&vec!["c", &format!("container: {name}")], None).unwrap();

    p.exp_regex(&format!(r#""docker" "exec" "{name}" "sh" "-c" "(?:bash|ash|sh)""#)).unwrap();
    p.exp_regex(&format!(r#""docker" "exec" "-it" "{name}" "(?:bash|ash|sh)""#)).unwrap();
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_connect_to_server_container() {
    setup().await;
    delete_all().await.unwrap();

    let name = "connect_to_server_container";
    let user = "user_connect_to_server_container";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();

    // scan
    let mut p = spawn_test(&vec!["s", &format!("server: {name}"), "-f", "fake-container-1", "-f", "fake-container-2"], None).unwrap();
    exp_string_assert(p.exp_string("finished"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // conenct
    let mut p = spawn_test(&vec!["c", &format!("server container: {name}: fake-container-1")], None).unwrap();

    p.exp_regex(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "docker exec fake-container-1 sh -c (?:bash|ash|sh)""#)).unwrap();
    p.exp_regex(&format!(r#""ssh" "-t" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "docker exec -it fake-container-1 (?:bash|ash|sh)""#)).unwrap();
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_show_key() {
    setup().await;
    delete_all().await.unwrap();

    let name = "show_key";
    let user = "user_show_key";
    let host = "192.168.1.1";
    let port = "11";
    let key = "user_show_key_key";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: Some("".to_string())
    }).await.unwrap();

    let key_location = format!("{}.pub", &key_dir().unwrap().join(&key).display());
    let key_content = fs::read_to_string(&key_location).unwrap();

    // show key
    let mut p = spawn_test(&vec!["sk", &format!("server: {name}")], None).unwrap();
    exp_string_assert(p.exp_string(&format!("Key location: {}", &key_location)), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&key_content.trim()), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_show_key_default_key() {
    setup().await;
    delete_all().await.unwrap();

    let name = "show_key_default_key";
    let user = "user_show_key_default_key";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();

    // show key
    let mut p = spawn_test(&vec!["sk", &format!("server: {name}")], None).unwrap();
    exp_string_assert(p.exp_string("Info: Use default keys"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_show_detail_container() {
    setup().await;
    delete_all().await.unwrap();
    
    let name = "show_detail_container";

    // show detail
    let mut p = spawn_test(&vec!["si", &format!("container: {name}")], None).unwrap();
    exp_string_assert(p.exp_string(&format!(r#""docker" "ps" "-f" "name=^{name}$" "--format" "CONTAINER:\n  Name: {{{{.Names}}}}\n  ID: {{{{.ID}}}}\n  Image: {{{{.Image}}}}\n  Status: {{{{.Status}}}}\n  Ports: {{{{.Ports}}}}""#)), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_show_detail_server_default_key() {
    setup().await;
    delete_all().await.unwrap();

    let name = "show_detail_server_default_key";
    let user = "user_show_detail_server_default_key";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();

    // show detail
    let mut p = spawn_test(&vec!["si", &format!("server: {name}")], None).unwrap();
    exp_string_assert(p.exp_string(&format!("Name: {name}")), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!("Address: {user}@{host}:{port}")), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!("Key: [default key]")), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_show_detail_server_custom_key() {
    setup().await;
    delete_all().await.unwrap();

    let name = "show_detail_server_custom_key";
    let user = "user_show_detail_server_custom_key";
    let host = "192.168.1.1";
    let port = "11";
    let key = "show_detail_server_custom_key_key";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: Some("".to_string())
    }).await.unwrap();

    // show detail
    let mut p = spawn_test(&vec!["si", &format!("server: {name}")], None).unwrap();
    exp_string_assert(p.exp_string(&format!("Name: {name}")), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!("Address: {user}@{host}:{port}")), &mut p, file!(), line!(), column!());
    exp_string_assert(p.exp_string(&format!("Key: {key}")), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}

#[tokio::test]
async fn test_show_detail_server_container() {
    setup().await;
    delete_all().await.unwrap();

    let name = "show_detail_server_container";
    let user = "user_show_detail_server_container";
    let host = "192.168.1.1";
    let port = "11";
    let key = "";

    // create server
    node_service::add_server(AddServerServiceParams {
        name: name.to_string(),
        user: user.to_string(),
        host: host.to_string(),
        port: port.to_string(),
        key: key.to_string(),
        default_passphrase: None
    }).await.unwrap();

    // scan
    let mut p = spawn_test(&vec!["s", &format!("server: {name}"), "-f", "fake-container-1", "-f", "fake-container-2"], None).unwrap();
    exp_string_assert(p.exp_string("finished"), &mut p, file!(), line!(), column!());
    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());

    // show detail
    let mut p = spawn_test(&vec!["si", &format!("server container: {name}: fake-container-1")], None).unwrap();

    let expected1 = format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "docker ps -f name='^fake-container-1$' --format 'CONTAINER:\n    Name: {{{{.Names}}}}\n    ID: {{{{.ID}}}}\n    Image: {{{{.Image}}}}\n    Status: {{{{.Status}}}}\n    Ports: {{{{.Ports}}}}'""#);
    let expected2 = format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "docker ps -f name=\'^fake-container-1$\' --format \'CONTAINER:\n    Name: {{{{.Names}}}}\n    ID: {{{{.ID}}}}\n    Image: {{{{.Image}}}}\n    Status: {{{{.Status}}}}\n    Ports: {{{{.Ports}}}}\'""#);
    p.exp_regex(&format!("(?:{}|{})", regex::escape(&expected1), regex::escape(&expected2))).unwrap();

    
    eof_assert(p.exp_eof(), &mut p, file!(), line!(), column!());
}