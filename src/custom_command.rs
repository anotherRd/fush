use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(alias = "c", about = "(alias: c) - connect to server or container\nusage: fush connect \"container: project-nginx-1\" or wihtout argument to use selector\n")]
    Conenct {
        arg: Option<String>,
    },
    #[command(alias = "a", about = "(alias: a) - add new server\n")]
    Add,
    #[command(alias = "e", about = "(alias: e) - edit server\nusage: fush edit \"server: development1\" or wihtout argument to use selector\n")]
    Edit {
        arg: Option<String>,
    },
    #[command(alias = "d", about = "(alias: d) - delete one or more servers\nusage: fush delete \"server: development1\" \"server: development2\" or wihtout argument to use selector\n")]
    Delete {
        args: Vec<String>,
    },
    #[command(alias = "s", about = "(alias: s) - scan one or more servers for available containers\nusage: fush scan \"server: development1\" \"server: development2\" or wihtout argument to use selector\n")]
    Scan {
        args: Vec<String>,
        #[arg(short, long, hide = true)]
        fake_container: Vec<String>,
    },
    #[command(alias = "S", about = "(alias: S) - scan-all servers for available containers\n")]
    ScanAll {
        #[arg(short, long, hide = true)]
        fake_container: Vec<String>,
    },
    #[command(alias = "sk", about = "(alias: sk) - show public key path and content used by server\nusage: fush show-key \"server: development1\" or wihtout argument to use selector\n")]
    ShowKey {
        arg: Option<String>,
    },
    #[command(alias = "sd", about = "(alias: sd) - show detail of a node\nusage: fush show-detail \"server: development1\" or wihtout argument to use selector\n")]
    ShowDetail {
        arg: Option<String>,
    },
    #[cfg(debug_assertions)]
    Test,
}