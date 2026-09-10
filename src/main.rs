use clap::{CommandFactory, Parser};
use fush::custom_command::{Cli, Commands};
use fush::helper::general_helper::{is_test};
use fush::helper::node_helper::{select_multi_server, select_node, select_server};
use fush::interaction_function::{add_server, connect, delete_server, edit_server, scan_server_container, show_info, show_key};
use fush::setup::main_setup;
use std::{vec};

#[cfg(debug_assertions)]
async fn test () -> Result<(), Box<dyn std::error::Error>> {
    use std::process::{Command};
    let status = Command::new("cargo")
        .env("FUSH_TEST", "1")
        .args(["test", "--", "--test-threads", "1"])
        .status()?;

    if !status.success() {
        return Err(format!("{status}").into());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // for test
    #[cfg(debug_assertions)]
    if let Some(command) = Cli::parse().command {
        if command == Commands::Test {
            return test().await;
        }
    }

    main_setup().await?;

    // command
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Conenct { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_node("Select a node to connect").await?;
            }
            connect(selected).await?;
        },
        Some(Commands::Add) => {
            add_server().await?;
        },
        Some(Commands::Edit { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_server("Select a server to edit").await?;
            }
            edit_server(selected).await?;
        },
        Some(Commands::Delete { args }) => {
            let selections;
            if !args.is_empty() {
                selections = args;
            } else {
                selections = select_multi_server("Select server(s) to delete").await?;
            }
            delete_server(selections).await?;
        },
        Some(Commands::Scan { args, mut fake_container }) => {
            if !is_test() {
                fake_container = vec![];
            }

            let selections;
            if !args.is_empty() {
                selections = args;
            } else {
                selections = select_multi_server("Select server(s) to scan").await?;
            }
            scan_server_container(false, selections, fake_container).await?;
        },
        Some(Commands::ScanAll {mut fake_container}) => {
            if !is_test() {
                fake_container = vec![];
            }

            scan_server_container(true, vec![], fake_container).await?;
        },
        Some(Commands::ShowKey { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_server("Select a server to show the used key").await?;
            }
            show_key(selected).await?;
        }
        Some(Commands::ShowInfo { arg }) => {
            let selected;
            if let Some(selected_value) = arg {
                selected = selected_value;
            } else {
                selected = select_node("Select a node to show info").await?;
            }
            show_info(selected).await?;
        },
        #[cfg(debug_assertions)]
        Some(Commands::Test) =>  {
            
        }
        None => Cli::command().print_help()?
    }

    Ok(())
}