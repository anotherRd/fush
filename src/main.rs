pub mod config;

use crate::config::{init_config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // init config
    let _ = init_config()?;

    Ok(())
}