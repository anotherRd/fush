use fush::{helper::key_pair_exists, service::node_service, service_params::node_service_params::AddServerServiceParams};
use rexpect::{error::Error, session::spawn_command};
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

fn spawn_test(args: &Vec<&str>, timeout_ms: Option<u64>) -> Result<rexpect::session::PtySession, Error> {
    let mut cmd = Command::new(BINARY);
    cmd.env("FUSH_TEST", "1");
    cmd.args(args);

    spawn_command(cmd, timeout_ms)
}

#[tokio::test]
async fn test_add_server() {
    setup().await;

    let name = "add_server";
    let user = "user_add_server";
    let host = "192.168.1.1";
    let port = "33";
    let key = "add_server_key";

    let mut p = spawn_test(&vec!["a"], Some(5_000)).unwrap();

    p.exp_string("Name: ").unwrap();
    p.send_line(&name).unwrap();

    p.exp_string("User: ").unwrap();
    p.send_line(&user).unwrap();

    p.exp_string("Host: ").unwrap();
    p.send_line(&host).unwrap();

    p.exp_string("Port (22): ").unwrap();
    p.send_line(&port).unwrap();

    p.exp_string("Custom key name (use default key/password if empty): ").unwrap();
    p.send_line(&key).unwrap();

    p.exp_string("Enter passphrase").unwrap();
    p.send_line("").unwrap();
    
    p.exp_string("Enter same passphrase again:").unwrap();
    p.send_line("").unwrap();
    p.exp_string("Success:").unwrap();
    p.exp_eof().unwrap();

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

    let name = "add_server_default_value";
    let user = "user_add_server_default_value";
    let host = "192.168.1.1";
    let port = "";
    let key = "";

    let mut p = spawn_test(&vec!["a"], Some(5_000)).unwrap();

    p.exp_string("Name: ").unwrap();
    p.send_line(&name).unwrap();

    p.exp_string("User: ").unwrap();
    p.send_line(&user).unwrap();

    p.exp_string("Host: ").unwrap();
    p.send_line(&host).unwrap();

    p.exp_string("Port (22): ").unwrap();
    p.send_line(&port).unwrap();

    p.exp_string("Custom key name (use default key/password if empty): ").unwrap();
    p.send_line(&key).unwrap();

    p.exp_string("Success:").unwrap();
    p.exp_eof().unwrap();

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
    let mut p = spawn_test(&vec!["a"], Some(5_000)).unwrap();

    p.exp_string("Name: ").unwrap();
    p.send_line(&name).unwrap();

    p.exp_string("User: ").unwrap();
    p.send_line(&user).unwrap();

    p.exp_string("Host: ").unwrap();
    p.send_line(&host).unwrap();

    p.exp_string("Port (22): ").unwrap();
    p.send_line(&port).unwrap();

    p.exp_string("Custom key name (use default key/password if empty): ").unwrap();
    p.send_line(&key).unwrap();

    p.exp_string("Error: Database(SqliteError { code: 2067").unwrap();
    p.exp_eof().unwrap();
}

#[tokio::test]
async fn test_edit_server() {
    setup().await;

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

    let mut p = spawn_test(&vec!["e", &format!("server: {name_before}")], Some(5_000)).unwrap();

    p.exp_string(&format!("Name ({name_before}): ")).unwrap();
    p.send_line(&name_after).unwrap();

    p.exp_string(&format!("User ({user_before}): ")).unwrap();
    p.send_line(&user_after).unwrap();

    p.exp_string(&format!("Host ({host_before}): ")).unwrap();
    p.send_line(&host_after).unwrap();

    p.exp_string(&format!("Port ({port_before}): ")).unwrap();
    p.send_line(&port_after).unwrap();
    
    if key_before == "" {
        p.exp_string(&format!("Change key [y/n]?: ")).unwrap();
        p.send_line("y").unwrap();
    } else {
        p.exp_string(&format!("Change key ({key_before}) [y/n]?: ")).unwrap();
        p.send_line("y").unwrap();
    }

    p.exp_string("Custom key name (use default key/password if empty): ").unwrap();
    p.send_line(&key_after).unwrap();

    p.exp_string("Enter passphrase").unwrap();
    p.send_line("").unwrap();
    
    p.exp_string("Enter same passphrase again:").unwrap();
    p.send_line("").unwrap();

    p.exp_string("Success:").unwrap();
    p.exp_eof().unwrap();

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

    let mut p = spawn_test(&vec!["e", &format!("server: {name_before}")], Some(5_000)).unwrap();

    p.exp_string(&format!("Name ({name_before}): ")).unwrap();
    p.send_line(&name_after).unwrap();

    p.exp_string(&format!("User ({user_before}): ")).unwrap();
    p.send_line(&user_after).unwrap();

    p.exp_string(&format!("Host ({host_before}): ")).unwrap();
    p.send_line(&host_after).unwrap();

    p.exp_string(&format!("Port ({port_before}): ")).unwrap();
    p.send_line(&port_after).unwrap();
    
    if key_before == "" {
        p.exp_string(&format!("Change key [y/n]?: ")).unwrap();
        p.send_line("n").unwrap();
    } else {
        p.exp_string(&format!("Change key ({key_before}) [y/n]?: ")).unwrap();
        p.send_line("n").unwrap();
    }

    p.exp_string("Success:").unwrap();
    p.exp_eof().unwrap();

    // check saved data
    let saved_data = node_service::find_node_by_name(&name_before).await.unwrap();
    assert_eq!(name_before, saved_data.name);
    assert_eq!(format!("{user_before}@{host_before}:{port_before}"), saved_data.address);
    assert_eq!(None, saved_data.key);
    assert_eq!(None, saved_data.parent_id);
}

#[tokio::test]
async fn test_delete_server() {
    setup().await;

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
    let mut p = spawn_test(&vec!["d", &format!("server: {name}")], Some(5_000)).unwrap();

    p.exp_string("Are you sure [y/n]?").unwrap();
    p.send_line("y").unwrap();

    p.exp_string("Success:").unwrap();
    p.exp_eof().unwrap();

    // check saved data
    let saved_data = node_service::get_node_by_names(&name).await.unwrap();
    assert!(saved_data.is_empty());
}

#[tokio::test]
async fn test_delete_server_cancel() {
    setup().await;

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
    let mut p = spawn_test(&vec!["d", &format!("server: {name}")], Some(5_000)).unwrap();

    p.exp_string("Are you sure [y/n]?").unwrap();
    p.send_line("n").unwrap();
    p.exp_eof().unwrap();

    // check saved data
    let saved_data = node_service::get_node_by_names(&name).await.unwrap();
    assert_eq!(false, saved_data.is_empty());
}

#[tokio::test]
async fn test_scan_server_container_selecttion() {
    setup().await;

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
    let mut p = spawn_test(&vec!["s", &format!("server: {name}"), "-f", "fake-container-1", "-f", "fake-container-2"], Some(5_000)).unwrap();

    p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "-t" "docker ps --format {{{{.Names}}}}""#)).unwrap();
    p.exp_string("finished").unwrap();
    p.exp_eof().unwrap();

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
    let mut p = spawn_test(&vec!["S", "-f", "fake-container-all-1", "-f", "fake-container-all-2"], Some(5_000)).unwrap();

    p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "-t""#)).unwrap();
    p.exp_string(&format!(r#""docker ps --format {{{{.Names}}}}""#)).unwrap();
    p.exp_string("finished").unwrap();
    p.exp_eof().unwrap();

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

    let key_dir = key_dir().unwrap();

    // connect
    let mut p = spawn_test(&vec!["c", &format!("server: {name}")], Some(5_000)).unwrap();

    p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "-i" "{key_dir}/{key}""#)).unwrap();
    p.exp_eof().unwrap();
}

#[tokio::test]
async fn test_connect_to_server_default_key() {
    setup().await;

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
    let mut p = spawn_test(&vec!["c", &format!("server: {name}")], Some(5_000)).unwrap();

    p.exp_string(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}""#)).unwrap();
    p.exp_eof().unwrap();
}

#[tokio::test]
async fn test_connect_to_container() {
    setup().await;

    let name = "connect_to_container";

    // conenct
    let mut p = spawn_test(&vec!["c", &format!("container: {name}")], Some(5_000)).unwrap();

    p.exp_regex(&format!(r#""docker" "exec" "{name}" "sh" "-c" "(?:bash|ash|sh)""#)).unwrap();
    p.exp_regex(&format!(r#""docker" "exec" "-it" "{name}" "(?:bash|ash|sh)""#)).unwrap();
    p.exp_eof().unwrap();
}

#[tokio::test]
async fn test_connect_to_server_container() {
    setup().await;

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
    let mut p = spawn_test(&vec!["s", &format!("server: {name}"), "-f", "fake-container-1", "-f", "fake-container-2"], Some(5_000)).unwrap();
    p.exp_string("finished").unwrap();
    p.exp_eof().unwrap();

    // conenct
    let mut p = spawn_test(&vec!["c", &format!("server container: {name}: fake-container-1")], Some(5_000)).unwrap();

    p.exp_regex(&format!(r#""ssh" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "docker exec fake-container-1 sh -c (?:bash|ash|sh)""#)).unwrap();
    p.exp_regex(&format!(r#""ssh" "-t" "-o" "ConnectTimeout=5" "{user}@{host}" "-p" "{port}" "docker exec -it fake-container-1 (?:bash|ash|sh)""#)).unwrap();
    p.exp_eof().unwrap();
}

#[tokio::test]
async fn test_show_key() {
    setup().await;

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

    let key_location = format!("{}/{}.pub", &key_dir().unwrap(), &key);
    let key_content = fs::read_to_string(&key_location).unwrap();

    // scan
    let mut p = spawn_test(&vec!["sk", &format!("server: {name}")], Some(5_000)).unwrap();
    p.exp_string(&format!("Key location: {}", &key_location)).unwrap();
    p.exp_string(&key_content.trim()).unwrap();
    p.exp_eof().unwrap();
}

#[tokio::test]
async fn test_show_key_default_key() {
    setup().await;

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

    // scan
    let mut p = spawn_test(&vec!["sk", &format!("server: {name}")], Some(5_000)).unwrap();
    p.exp_string("Info: Use default keys").unwrap();
    p.exp_eof().unwrap();
}